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

/// Inbound commands a client may send after authenticating. Unknown frames are
/// ignored (forward-compatible). `watch`/`unwatch` drive incident presence.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientCommand {
    Watch { incident_id: Uuid },
    Unwatch { incident_id: Uuid },
    StatusTyping { incident_id: Uuid },
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

    // 3. Register with the hub and pump events to the socket. The team set is
    //    also kept locally to authorize presence/typing commands (step 4).
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let conn_id = state.events.register(user_id, teams.clone(), tx);

    let mut send_task = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            if sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    // 4. Handle inbound commands (presence) until the client closes or errors.
    //    Unparseable or unknown frames are ignored.
    let hub = state.events.clone();
    let incidents = state.incidents.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Text(text) => match serde_json::from_str::<ClientCommand>(text.as_str()) {
                    // Presence and typing are authorized in-band: a client may only
                    // watch or signal typing on an incident in a team it belongs to.
                    // Otherwise any authenticated socket could join the watcher
                    // roster of — and leak presence/typing for — a foreign team's
                    // incident, since the REST plane enforces this but the WS plane
                    // historically did not.
                    Ok(ClientCommand::Watch { incident_id }) => {
                        if let Ok(Some(incident)) = incidents.find_incident_by_id(incident_id).await
                        {
                            if teams.contains(&incident.team_id) {
                                hub.watch(conn_id, incident_id);
                            }
                        }
                    }
                    // Unwatch only ever removes this connection from a watcher set,
                    // so it is harmless even for an incident the user cannot see.
                    Ok(ClientCommand::Unwatch { incident_id }) => hub.unwatch(conn_id, incident_id),
                    Ok(ClientCommand::StatusTyping { incident_id }) => {
                        if let Ok(Some(incident)) = incidents.find_incident_by_id(incident_id).await
                        {
                            if teams.contains(&incident.team_id) {
                                use crate::domain::event::DomainEvent;
                                use crate::ports::EventPublisher;
                                let _ = hub
                                    .publish(DomainEvent::UserTyping {
                                        team_id: incident.team_id,
                                        incident_id,
                                        user_id,
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(_) => {}
                },
                _ => {}
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
