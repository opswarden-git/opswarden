// --- server/src/domain/private_message.rs ---
//
// A private message: a strictly bilateral, 1-to-1 direct message between two
// users. It is not tied to an incident, release or team — the conversation is
// identified solely by its two participants. The "may these two users talk"
// authorization (shared team) is a use-case concern; the domain only owns the
// message's own invariants (a non-blank, length-bounded body).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

/// Server-side body cap. Matches the timeline-entry limit: generous for a real
/// message, tight enough to refuse pasted documents.
pub const MAX_PRIVATE_MESSAGE_LEN: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
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
        let content = Self::validate_content(content)?;
        Ok(Self {
            id: Uuid::new_v4(),
            sender_id,
            recipient_id,
            content,
            created_at: Utc::now(),
        })
    }

    fn validate_content(content: impl Into<String>) -> Result<String, DomainError> {
        let content = content.into();
        let trimmed = content.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_PRIVATE_MESSAGE_LEN {
            return Err(DomainError::InvalidPrivateMessage);
        }
        Ok(trimmed.to_string())
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
}
