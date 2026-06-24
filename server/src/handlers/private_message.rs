// --- server/src/handlers/private_message.rs ---
//
// HTTP surface for private messages, both routes behind `require_auth`:
//   POST /api/private-messages       { recipient_id, content }
//   GET  /api/private-messages?peer_id=..&limit=..
// The authenticated user is always one participant (sender / requester); the
// other is taken from the request. Authorization and validation live in the
// use-cases, so these handlers only translate HTTP <-> command/result.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::private_message::{
    ListPrivateMessagesCommand, ListPrivateMessagesUseCase, SendPrivateMessageCommand,
    SendPrivateMessageUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::private_message::PrivateMessage;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct SendPrivateMessagePayload {
    pub recipient_id: Uuid,
    pub content: String,
}

#[derive(Serialize)]
pub struct PrivateMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<PrivateMessage> for PrivateMessageResponse {
    fn from(message: PrivateMessage) -> Self {
        Self {
            id: message.id,
            sender_id: message.sender_id,
            recipient_id: message.recipient_id,
            content: message.content,
            created_at: message.created_at,
        }
    }
}

pub async fn send_private_message(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<SendPrivateMessagePayload>,
) -> Result<(StatusCode, Json<PrivateMessageResponse>), DomainError> {
    let use_case = SendPrivateMessageUseCase::new(
        state.users.clone(),
        state.teams.clone(),
        state.private_messages.clone(),
        state.events.clone(),
    );

    let result = use_case
        .send(SendPrivateMessageCommand {
            sender_id: session.user_id,
            recipient_id: payload.recipient_id,
            content: payload.content,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(PrivateMessageResponse {
            id: result.message_id,
            sender_id: result.sender_id,
            recipient_id: result.recipient_id,
            content: result.content,
            created_at: result.created_at,
        }),
    ))
}

#[derive(Deserialize)]
pub struct ListPrivateMessagesQuery {
    pub peer_id: Uuid,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ListPrivateMessagesResponse {
    /// Newest first.
    pub messages: Vec<PrivateMessageResponse>,
}

pub async fn list_private_messages(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Query(query): Query<ListPrivateMessagesQuery>,
) -> Result<Json<ListPrivateMessagesResponse>, DomainError> {
    let use_case = ListPrivateMessagesUseCase::new(
        state.users.clone(),
        state.teams.clone(),
        state.private_messages.clone(),
    );

    let result = use_case
        .list(ListPrivateMessagesCommand {
            requester_id: session.user_id,
            peer_id: query.peer_id,
            limit: query.limit,
        })
        .await?;

    Ok(Json(ListPrivateMessagesResponse {
        messages: result
            .messages
            .into_iter()
            .map(PrivateMessageResponse::from)
            .collect(),
    }))
}
