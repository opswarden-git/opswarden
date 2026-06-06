// --- server/src/handlers/ws.rs ---

use std::collections::HashSet;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

/// `GET /ws` — upgrade to a WebSocket. This route is public: authentication
/// happens in-band via the first message (browsers cannot set an Authorization
/// header on the WS handshake), so the connection is anonymous until it sends a
/// valid `{"type":"auth","token":"..."}`.
pub async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

#[derive(Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    kind: String,
    token: String,
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. First-message authentication. Anything other than a valid auth frame
    //    closes the connection.
    let user_id = loop {
        match receiver.next().await {
            Some(Ok(Message::Text(text))) => match authenticate(text.as_str(), &state).await {
                Some(uid) => break uid,
                None => {
                    let _ = sender.send(Message::Close(None)).await;
                    return;
                }
            },
            // Ignore ping/binary frames sent before authenticating.
            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_))) => continue,
            _ => return,
        }
    };

    // 2. Scope the connection to the teams the user belongs to.
    let teams: HashSet<Uuid> = match state.teams.list_team_ids_for_user(user_id).await {
        Ok(ids) => ids.into_iter().collect(),
        Err(_) => return,
    };

    // 3. Register with the hub and pump events to the socket.
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let conn_id = state.events.register(user_id, teams, tx);

    let mut send_task = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            if sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    // 4. Drain inbound until the client closes or errors (PR1 has no inbound
    //    commands yet — presence arrives in PR2).
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    state.events.unregister(conn_id);
}

async fn authenticate(text: &str, state: &AppState) -> Option<Uuid> {
    let auth: AuthMessage = serde_json::from_str(text).ok()?;
    if auth.kind != "auth" {
        return None;
    }
    let claims = state.tokens.verify_token(&auth.token).ok()?;
    if state
        .token_revocations
        .is_revoked(&auth.token)
        .await
        .unwrap_or(true)
    {
        return None;
    }
    Some(claims.user_id)
}
