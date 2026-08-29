// --- server/src/handlers/ws.rs ---

use std::collections::HashSet;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header::ORIGIN, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::timeout;
use uuid::Uuid;

use crate::adapters::ws::hub::OUTBOUND_QUEUE_CAPACITY;
use crate::domain::capabilities::derive_capabilities;
use crate::ports::TokenClaims;
use crate::AppState;

const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PREAUTHENTICATION_FRAMES: usize = 8;

/// `GET /ws` — upgrade to a WebSocket. This route is public: authentication
/// happens in-band via the first message (browsers cannot set an Authorization
/// header on the WS handshake), so the connection is anonymous until it sends a
/// valid `{"type":"auth","token":"..."}`.
pub async fn ws_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !origin_is_allowed(&headers, &state.config.ws_allowed_origins) {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

fn origin_is_allowed(headers: &HeaderMap, allowed_origins: &[String]) -> bool {
    let mut origins = headers.get_all(ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    origin
        .to_str()
        .ok()
        .is_some_and(|origin| allowed_origins.iter().any(|allowed| allowed == origin))
}

#[derive(Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    kind: String,
    token: String,
}

/// Inbound commands a client may send after authenticating. Unknown frames are
/// ignored (forward-compatible). `watch`/`unwatch` drive incident presence;
/// `refresh_teams` re-resolves the connection's team scope after the user
/// created, joined, left, or deleted a team (the scope is otherwise fixed at
/// auth time, which would leave team presence and authz stale until reconnect).
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientCommand {
    Watch { incident_id: Uuid },
    Unwatch { incident_id: Uuid },
    StatusTyping { incident_id: Uuid },
    Cursor { incident_id: Uuid, x: f64, y: f64 },
    WatchPrivateMessage { peer_id: Uuid },
    UnwatchPrivateMessage { peer_id: Uuid },
    PrivateMessageTyping { peer_id: Uuid },
    RefreshTeams,
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. First-message authentication. Anything other than a valid auth frame
    //    closes the connection.
    let claims = match timeout(
        AUTHENTICATION_TIMEOUT,
        receive_authentication(&mut receiver, &state),
    )
    .await
    {
        Ok(Some(claims)) => claims,
        Ok(None) | Err(_) => {
            let _ = sender.send(Message::Close(None)).await;
            return;
        }
    };
    let user_id = claims.user_id;
    let session_lifetime = session_lifetime(claims.expires_at, state.clock.now());
    if session_lifetime.is_zero() {
        let _ = sender.send(Message::Close(None)).await;
        return;
    }

    // 2. Scope the connection to the teams the user belongs to.
    let teams: HashSet<Uuid> = match state.teams.list_team_ids_for_user(user_id).await {
        Ok(ids) => ids.into_iter().collect(),
        Err(_) => return,
    };

    // 3. Register with the hub and pump events to the socket. The team set is
    //    also kept locally to authorize presence/typing commands (step 4).
    let (tx, mut rx) = mpsc::channel::<String>(OUTBOUND_QUEUE_CAPACITY);
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
    let teams_repo = state.teams.clone();
    let mut recv_task = tokio::spawn(async move {
        // Owned, mutable copy of the team scope: kept in sync with the hub on
        // `refresh_teams` so in-band authz (watch/typing) also follows membership.
        let mut teams = teams;
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
                    Ok(ClientCommand::Cursor { incident_id, x, y }) => {
                        hub.cursor(conn_id, incident_id, x, y)
                    }
                    Ok(ClientCommand::WatchPrivateMessage { peer_id }) => {
                        if peer_id != user_id
                            && matches!(
                                crate::app::private_message::users_share_team(
                                    teams_repo.as_ref(),
                                    user_id,
                                    peer_id,
                                )
                                .await,
                                Ok(true)
                            )
                        {
                            hub.watch_private_message(conn_id, peer_id);
                        }
                    }
                    Ok(ClientCommand::UnwatchPrivateMessage { peer_id }) => {
                        hub.unwatch_private_message(conn_id, peer_id)
                    }
                    Ok(ClientCommand::PrivateMessageTyping { peer_id }) => {
                        if peer_id != user_id
                            && matches!(
                                crate::app::private_message::users_share_team(
                                    teams_repo.as_ref(),
                                    user_id,
                                    peer_id,
                                )
                                .await,
                                Ok(true)
                            )
                        {
                            hub.private_message_typing(conn_id, peer_id);
                        }
                    }
                    // Re-resolve the team scope from the database (the authority)
                    // and update both the hub (presence routing) and the local
                    // authz copy. The hub re-broadcasts presence for every team
                    // the change touched.
                    Ok(ClientCommand::RefreshTeams) => {
                        if let Ok(ids) = teams_repo.list_team_ids_for_user(user_id).await {
                            let new_teams: HashSet<Uuid> = ids.into_iter().collect();
                            teams = new_teams.clone();
                            hub.refresh_teams(conn_id, new_teams);
                        }
                    }
                    Ok(ClientCommand::StatusTyping { incident_id }) => {
                        if let Ok(Some(incident)) = incidents.find_incident_by_id(incident_id).await
                        {
                            let may_type = teams.contains(&incident.team_id)
                                && matches!(
                                    teams_repo
                                        .find_member_role(incident.team_id, user_id)
                                        .await,
                                    Ok(Some(role)) if derive_capabilities(role).can_signal_typing
                                );
                            if may_type {
                                hub.typing(conn_id, incident_id);
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
        _ = tokio::time::sleep(session_lifetime) => {
            // Dropping the hub's bounded sender ends the outbound task and
            // closes the socket. The peer cannot retain a session beyond the
            // JWT's absolute expiry even if it remains otherwise idle.
            state.events.unregister(conn_id);
            recv_task.abort();
            let _ = send_task.await;
        }
    }

    state.events.unregister(conn_id);
}

async fn receive_authentication(
    receiver: &mut futures_util::stream::SplitStream<WebSocket>,
    state: &AppState,
) -> Option<TokenClaims> {
    let mut frames_seen = 0;
    loop {
        frames_seen += 1;
        if frames_seen > MAX_PREAUTHENTICATION_FRAMES {
            return None;
        }
        match receiver.next().await {
            Some(Ok(Message::Text(text))) => return authenticate(text.as_str(), state).await,
            // A few transport-level frames are tolerated, but they cannot
            // extend the outer authentication deadline or keep the socket
            // anonymous indefinitely.
            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_))) => continue,
            _ => return None,
        }
    }
}

async fn authenticate(text: &str, state: &AppState) -> Option<TokenClaims> {
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
    if state.users.find_by_id(claims.user_id).await.ok()?.is_none() {
        return None;
    }
    Some(claims)
}

fn session_lifetime(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    expires_at
        .signed_duration_since(now)
        .to_std()
        .unwrap_or(Duration::ZERO)
}

#[cfg(test)]
mod tests {
    use axum::http::{header::ORIGIN, HeaderMap, HeaderValue};
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    use super::{origin_is_allowed, session_lifetime};

    fn allowlist() -> Vec<String> {
        vec![
            "https://app.opswarden.dev".to_string(),
            "http://localhost:4242".to_string(),
        ]
    }

    #[test]
    fn accepts_exact_browser_origin_and_originless_native_client() {
        let mut headers = HeaderMap::new();
        assert!(origin_is_allowed(&headers, &allowlist()));

        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://app.opswarden.dev"),
        );
        assert!(origin_is_allowed(&headers, &allowlist()));
    }

    #[test]
    fn rejects_cross_site_null_and_multiple_origins() {
        for origin in [
            "https://attacker.example",
            "null",
            "https://app.opswarden.dev/",
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
            assert!(!origin_is_allowed(&headers, &allowlist()));
        }

        let mut headers = HeaderMap::new();
        headers.append(
            ORIGIN,
            HeaderValue::from_static("https://app.opswarden.dev"),
        );
        headers.append(ORIGIN, HeaderValue::from_static("https://attacker.example"));
        assert!(!origin_is_allowed(&headers, &allowlist()));
    }

    #[test]
    fn session_lifetime_uses_the_absolute_jwt_expiry() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        assert_eq!(
            session_lifetime(now + ChronoDuration::seconds(30), now),
            std::time::Duration::from_secs(30)
        );
        assert!(session_lifetime(now - ChronoDuration::seconds(1), now).is_zero());
    }
}
