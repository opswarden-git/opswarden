// --- server/src/domain/private_message.rs ---
//
// A private message: a strictly bilateral, 1-to-1 direct message between two
// users. It is not tied to an incident, release or team — the conversation is
// identified solely by its two participants. The "may these two users talk"
// authorization (shared team) is a use-case concern; the domain only owns the
// message's own invariants (a non-blank, length-bounded body).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::conversation::{normalize_message_content, validate_attachments, MessageAttachment};
use super::error::DomainError;

/// Server-side body cap. Matches the timeline-entry limit: generous for a real
/// message, tight enough to refuse pasted documents.
pub use super::conversation::{
    MAX_MESSAGE_ATTACHMENTS as MAX_PRIVATE_MESSAGE_ATTACHMENTS,
    MAX_MESSAGE_ATTACHMENTS_TOTAL_BYTES as MAX_PRIVATE_MESSAGE_ATTACHMENTS_TOTAL_BYTES,
    MAX_MESSAGE_ATTACHMENT_BYTES as MAX_PRIVATE_MESSAGE_ATTACHMENT_BYTES,
    MAX_MESSAGE_LEN as MAX_PRIVATE_MESSAGE_LEN,
};
pub type PrivateMessageAttachment = MessageAttachment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub attachments: Vec<PrivateMessageAttachment>,
}

impl PrivateMessage {
    /// Build a validated message. The directed `(sender, recipient)` pair is kept
    /// as authored; reads later fetch both directions of the pair. Blank or
    /// oversized content is rejected with `InvalidPrivateMessage`.
    pub fn new(
        sender_id: Uuid,
        recipient_id: Uuid,
        content: impl Into<String>,
    ) -> Result<Self, DomainError> {
        Self::new_with_attachments(sender_id, recipient_id, content, Vec::new())
    }

    pub fn new_with_attachments(
        sender_id: Uuid,
        recipient_id: Uuid,
        content: impl Into<String>,
        attachments: Vec<(String, String, Vec<u8>)>,
    ) -> Result<Self, DomainError> {
        let has_attachments = !attachments.is_empty();
        let content = normalize_message_content(content, has_attachments)
            .ok_or(DomainError::InvalidPrivateMessage)?;
        let attachments = validate_attachments(attachments)
            .ok_or(DomainError::InvalidPrivateMessageAttachment)?;

        let message_id = Uuid::new_v4();
        let created_at = Utc::now();
        let attachments = attachments
            .into_iter()
            .map(|attachment| {
                let size_bytes = attachment.content.len();
                PrivateMessageAttachment {
                    id: Uuid::new_v4(),
                    message_id,
                    file_name: attachment.file_name,
                    media_type: attachment.media_type,
                    size_bytes,
                    content: attachment.content,
                    created_at,
                }
            })
            .collect();

        Ok(Self {
            id: message_id,
            sender_id,
            recipient_id,
            content,
            created_at,
            edited_at: None,
            attachments,
        })
    }

    pub fn validate_edited_content(content: impl Into<String>) -> Result<String, DomainError> {
        normalize_message_content(content, false).ok_or(DomainError::InvalidPrivateMessage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_keeps_trimmed_content() {
        let msg = PrivateMessage::new(Uuid::new_v4(), Uuid::new_v4(), "  on my way  ").unwrap();
        assert_eq!(msg.content, "on my way");
    }

    #[test]
    fn blank_message_is_rejected() {
        let result = PrivateMessage::new(Uuid::new_v4(), Uuid::new_v4(), "   ");
        assert_eq!(result.unwrap_err(), DomainError::InvalidPrivateMessage);
    }

    #[test]
    fn oversized_message_is_rejected() {
        let result = PrivateMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "x".repeat(MAX_PRIVATE_MESSAGE_LEN + 1),
        );
        assert_eq!(result.unwrap_err(), DomainError::InvalidPrivateMessage);
    }

    #[test]
    fn max_length_message_is_accepted() {
        let result = PrivateMessage::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "x".repeat(MAX_PRIVATE_MESSAGE_LEN),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn attachment_can_be_the_whole_message() {
        let message = PrivateMessage::new_with_attachments(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "",
            vec![(
                "runbook.pdf".into(),
                "application/pdf".into(),
                vec![1, 2, 3],
            )],
        )
        .unwrap();
        assert!(message.content.is_empty());
        assert_eq!(message.attachments[0].message_id, message.id);
    }

    #[test]
    fn active_or_oversized_attachment_is_rejected() {
        for (media_type, bytes) in [
            ("text/html", vec![1]),
            (
                "image/png",
                vec![0; MAX_PRIVATE_MESSAGE_ATTACHMENT_BYTES + 1],
            ),
        ] {
            assert_eq!(
                PrivateMessage::new_with_attachments(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    "file",
                    vec![("file".into(), media_type.into(), bytes)],
                )
                .unwrap_err(),
                DomainError::InvalidPrivateMessageAttachment
            );
        }
    }
}
