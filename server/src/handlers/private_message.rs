// --- server/src/handlers/private_message.rs ---
//
// Authenticated HTTP surface for bilateral messaging. JSON carries small,
// bounded base64 attachments on send; downloads remain authorization-gated and
// are served as inert attachments with nosniff.

use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{Response, StatusCode},
    Json,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::private_message::{
    EditPrivateMessageCommand, EditPrivateMessageUseCase, GetPrivateMessageAttachmentUseCase,
    ListPrivateMessagesCommand, ListPrivateMessagesUseCase, ListUnreadPrivateMessagesUseCase,
    MarkPrivateMessageReadCommand, MarkPrivateMessageReadUseCase, SendPrivateMessageCommand,
    SendPrivateMessageResult, SendPrivateMessageUseCase, TogglePrivateMessageReactionCommand,
    TogglePrivateMessageReactionUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::private_message::PrivateMessage;
use crate::domain::{ConversationFeature, ConversationScope};
use crate::handlers::conversation::{
    attachment_download_response, AttachmentResponse, ConversationCursorResponse, ReactionResponse,
};
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct SendPrivateMessageAttachmentPayload {
    pub file_name: String,
    pub media_type: String,
    pub data_base64: String,
}

#[derive(Deserialize)]
pub struct SendPrivateMessagePayload {
    pub recipient_id: Uuid,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<SendPrivateMessageAttachmentPayload>,
}

#[derive(Serialize)]
pub struct PrivateMessageResponse {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub attachments: Vec<AttachmentResponse>,
    pub reactions: Vec<ReactionResponse>,
}

impl From<PrivateMessage> for PrivateMessageResponse {
    fn from(message: PrivateMessage) -> Self {
        Self {
            id: message.id,
            sender_id: message.sender_id,
            recipient_id: message.recipient_id,
            content: message.content,
            created_at: message.created_at,
            edited_at: message.edited_at,
            attachments: message.attachments.into_iter().map(Into::into).collect(),
            reactions: message.reactions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SendPrivateMessageResult> for PrivateMessageResponse {
    fn from(message: SendPrivateMessageResult) -> Self {
        Self {
            id: message.message_id,
            sender_id: message.sender_id,
            recipient_id: message.recipient_id,
            content: message.content,
            created_at: message.created_at,
            edited_at: None,
            attachments: message.attachments.into_iter().map(Into::into).collect(),
            reactions: Vec::new(),
        }
    }
}

pub async fn send_private_message(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<SendPrivateMessagePayload>,
) -> Result<(StatusCode, Json<PrivateMessageResponse>), DomainError> {
    let attachments = payload
        .attachments
        .into_iter()
        .map(|attachment| {
            base64::engine::general_purpose::STANDARD
                .decode(attachment.data_base64)
                .map(|content| (attachment.file_name, attachment.media_type, content))
                .map_err(|_| DomainError::InvalidPrivateMessageAttachment)
        })
        .collect::<Result<Vec<_>, _>>()?;
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
            attachments,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

#[derive(Deserialize)]
pub struct ConversationCursorQuery {
    pub peer_id: Uuid,
    pub limit: Option<u32>,
    pub before_created_at: Option<DateTime<Utc>>,
    pub before_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct ListPrivateMessagesResponse {
    /// Newest first within each page.
    pub messages: Vec<PrivateMessageResponse>,
    pub next_cursor: Option<ConversationCursorResponse>,
    pub features: Vec<ConversationFeature>,
}

pub async fn list_private_messages(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Query(query): Query<ConversationCursorQuery>,
) -> Result<Json<ListPrivateMessagesResponse>, DomainError> {
    let before = match (query.before_created_at, query.before_id) {
        (Some(at), Some(id)) => Some((at, id)),
        (None, None) => None,
        _ => return Err(DomainError::InvalidPrivateMessage),
    };
    let result = ListPrivateMessagesUseCase::new(
        state.users.clone(),
        state.teams.clone(),
        state.private_messages.clone(),
    )
    .list(ListPrivateMessagesCommand {
        requester_id: session.user_id,
        peer_id: query.peer_id,
        limit: query.limit,
        before,
    })
    .await?;

    Ok(Json(ListPrivateMessagesResponse {
        messages: result.messages.into_iter().map(Into::into).collect(),
        next_cursor: result
            .next_cursor
            .map(|(created_at, id)| ConversationCursorResponse { created_at, id }),
        features: ConversationScope::Direct {
            first_user: session.user_id,
            second_user: query.peer_id,
        }
        .features()
        .to_vec(),
    }))
}

#[derive(Deserialize)]
pub struct EditPrivateMessagePayload {
    pub content: String,
}

#[derive(Serialize)]
pub struct EditPrivateMessageResponse {
    pub content: String,
    pub edited_at: DateTime<Utc>,
}

pub async fn edit_private_message(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<EditPrivateMessagePayload>,
) -> Result<Json<EditPrivateMessageResponse>, DomainError> {
    let result = EditPrivateMessageUseCase::new(
        state.teams.clone(),
        state.private_messages.clone(),
        state.events.clone(),
    )
    .edit(EditPrivateMessageCommand {
        requester_id: session.user_id,
        message_id,
        content: payload.content,
    })
    .await?;
    Ok(Json(EditPrivateMessageResponse {
        content: result.content,
        edited_at: result.edited_at,
    }))
}

#[derive(Deserialize)]
pub struct TogglePrivateMessageReactionPayload {
    pub emoji: String,
}

#[derive(Serialize)]
pub struct TogglePrivateMessageReactionResponse {
    pub active: bool,
}

pub async fn toggle_private_message_reaction(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<TogglePrivateMessageReactionPayload>,
) -> Result<Json<TogglePrivateMessageReactionResponse>, DomainError> {
    let result = TogglePrivateMessageReactionUseCase::new(
        state.teams.clone(),
        state.private_messages.clone(),
        state.events.clone(),
    )
    .toggle(TogglePrivateMessageReactionCommand {
        requester_id: session.user_id,
        message_id,
        emoji: payload.emoji,
    })
    .await?;
    Ok(Json(TogglePrivateMessageReactionResponse {
        active: result.active,
    }))
}

pub async fn download_private_message_attachment(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(attachment_id): Path<Uuid>,
) -> Result<Response<Body>, DomainError> {
    let attachment = GetPrivateMessageAttachmentUseCase::new(
        state.teams.clone(),
        state.private_messages.clone(),
    )
    .get(attachment_id, session.user_id)
    .await?;
    Ok(attachment_download_response(attachment))
}

#[derive(Deserialize)]
pub struct MarkPrivateMessagesReadPayload {
    pub peer_id: Uuid,
    pub read_through: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UnreadPrivateMessagesResponse {
    pub unread_peer_ids: Vec<Uuid>,
}

pub async fn mark_private_messages_read(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<MarkPrivateMessagesReadPayload>,
) -> Result<StatusCode, DomainError> {
    MarkPrivateMessageReadUseCase::new(state.teams.clone(), state.private_messages.clone())
        .execute(MarkPrivateMessageReadCommand {
            viewer_id: session.user_id,
            peer_id: payload.peer_id,
            read_through: payload.read_through,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_unread_private_messages(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
) -> Result<Json<UnreadPrivateMessagesResponse>, DomainError> {
    let unread_peer_ids = ListUnreadPrivateMessagesUseCase::new(state.private_messages.clone())
        .execute(session.user_id)
        .await?;
    Ok(Json(UnreadPrivateMessagesResponse { unread_peer_ids }))
}
