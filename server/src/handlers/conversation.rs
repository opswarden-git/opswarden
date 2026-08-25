use axum::{
    body::Body,
    http::{header, HeaderValue, Response},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::conversation::{MessageAttachment, MessageReactionSummary};

#[derive(Serialize)]
pub struct AttachmentResponse {
    pub id: Uuid,
    pub file_name: String,
    pub media_type: String,
    pub size_bytes: usize,
}

impl From<MessageAttachment> for AttachmentResponse {
    fn from(attachment: MessageAttachment) -> Self {
        Self {
            id: attachment.id,
            file_name: attachment.file_name,
            media_type: attachment.media_type,
            size_bytes: attachment.size_bytes,
        }
    }
}

#[derive(Serialize)]
pub struct ReactionResponse {
    pub emoji: String,
    pub count: u64,
    pub reacted: bool,
}

impl From<MessageReactionSummary> for ReactionResponse {
    fn from(reaction: MessageReactionSummary) -> Self {
        Self {
            emoji: reaction.emoji,
            count: reaction.count,
            reacted: reaction.reacted,
        }
    }
}

#[derive(Serialize)]
pub struct ConversationCursorResponse {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

pub fn attachment_download_response(attachment: MessageAttachment) -> Response<Body> {
    let safe_name = attachment
        .file_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let mut response = Response::new(Body::from(attachment.content));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&attachment.media_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{safe_name}\""))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}
