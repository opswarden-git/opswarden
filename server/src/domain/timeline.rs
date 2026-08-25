use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::conversation::{
    normalize_message_content, validate_attachments, MessageAttachment, MAX_MESSAGE_LEN,
};
use super::error::DomainError;

pub const MAX_TIMELINE_ENTRY_LEN: usize = MAX_MESSAGE_LEN;
/// The one canonical reaction catalog. It is enforced by the domain, exposed by
/// `GET /reactions/available`, and consumed by every client.
pub const AVAILABLE_REACTIONS: [&str; 6] = ["👍", "👀", "✅", "🚨", "❤️", "🎉"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Option<Uuid>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /// `Some` once the entry has been edited; `created_at` is never moved.
    pub edited_at: Option<DateTime<Utc>>,
    pub attachments: Vec<MessageAttachment>,
}

impl TimelineEntry {
    pub fn new(
        incident_id: Uuid,
        author_id: Uuid,
        content: impl Into<String>,
    ) -> Result<Self, DomainError> {
        Self::new_with_attachments(incident_id, author_id, content, Vec::new())
    }

    pub fn new_with_attachments(
        incident_id: Uuid,
        author_id: Uuid,
        content: impl Into<String>,
        attachments: Vec<(String, String, Vec<u8>)>,
    ) -> Result<Self, DomainError> {
        let has_attachments = !attachments.is_empty();
        let content = normalize_message_content(content, has_attachments)
            .ok_or(DomainError::InvalidTimelineEntry)?;
        let attachments =
            validate_attachments(attachments).ok_or(DomainError::InvalidTimelineAttachment)?;
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        Ok(Self {
            id,
            incident_id,
            author_id: Some(author_id),
            content,
            created_at,
            edited_at: None,
            attachments: attachments
                .into_iter()
                .map(|attachment| MessageAttachment {
                    id: Uuid::new_v4(),
                    message_id: id,
                    file_name: attachment.file_name,
                    media_type: attachment.media_type,
                    size_bytes: attachment.content.len(),
                    content: attachment.content,
                    created_at,
                })
                .collect(),
        })
    }

    /// Replace the content with freshly validated text and stamp `edited_at`;
    /// `created_at` is preserved.
    pub fn edit(&mut self, content: impl Into<String>) -> Result<(), DomainError> {
        self.content = Self::validate_content(content)?;
        self.edited_at = Some(Utc::now());
        Ok(())
    }

    fn validate_content(content: impl Into<String>) -> Result<String, DomainError> {
        normalize_message_content(content, false).ok_or(DomainError::InvalidTimelineEntry)
    }
}

/// Accept only a member of the canonical catalog and return its normalized
/// representation. This prevents clients from creating a second, unbounded
/// reaction vocabulary.
pub fn validate_reaction_emoji(emoji: &str) -> Result<String, DomainError> {
    let trimmed = emoji.trim();
    if !AVAILABLE_REACTIONS.contains(&trimmed) {
        return Err(DomainError::InvalidReaction);
    }
    Ok(trimmed.to_string())
}

/// A single stored reaction (who reacted to which entry with what), read back
/// from persistence for aggregation into per-entry counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionRecord {
    pub entry_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_creation_keeps_trimmed_content() {
        let entry =
            TimelineEntry::new(Uuid::new_v4(), Uuid::new_v4(), "  investigate logs  ").unwrap();

        assert_eq!(entry.content, "investigate logs");
    }

    #[test]
    fn blank_entry_is_rejected() {
        let result = TimelineEntry::new(Uuid::new_v4(), Uuid::new_v4(), "   ");

        assert_eq!(result.unwrap_err(), DomainError::InvalidTimelineEntry);
    }

    #[test]
    fn oversized_entry_is_rejected() {
        let result = TimelineEntry::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "x".repeat(MAX_TIMELINE_ENTRY_LEN + 1),
        );

        assert_eq!(result.unwrap_err(), DomainError::InvalidTimelineEntry);
    }

    #[test]
    fn attachment_can_be_the_whole_operational_note() {
        let entry = TimelineEntry::new_with_attachments(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "",
            vec![("runbook.pdf".into(), "application/pdf".into(), vec![1, 2])],
        )
        .unwrap();

        assert!(entry.content.is_empty());
        assert_eq!(entry.attachments[0].file_name, "runbook.pdf");
    }

    #[test]
    fn edit_updates_trimmed_content_and_stamps_edited_at_keeping_created_at() {
        let mut entry = TimelineEntry::new(Uuid::new_v4(), Uuid::new_v4(), "first").unwrap();
        let created = entry.created_at;
        assert!(entry.edited_at.is_none());

        entry.edit("  second take  ").unwrap();

        assert_eq!(entry.content, "second take");
        assert_eq!(entry.created_at, created);
        assert!(entry.edited_at.is_some());
    }

    #[test]
    fn edit_rejects_blank_content() {
        let mut entry = TimelineEntry::new(Uuid::new_v4(), Uuid::new_v4(), "first").unwrap();

        let result = entry.edit("   ");

        assert_eq!(result.unwrap_err(), DomainError::InvalidTimelineEntry);
        assert_eq!(entry.content, "first");
        assert!(entry.edited_at.is_none());
    }

    #[test]
    fn reaction_catalog_contains_six_distinct_supported_emojis() {
        assert_eq!(AVAILABLE_REACTIONS.len(), 6);
        let mut distinct = AVAILABLE_REACTIONS.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), AVAILABLE_REACTIONS.len());

        for emoji in AVAILABLE_REACTIONS {
            assert_eq!(validate_reaction_emoji(emoji).unwrap(), emoji);
        }
    }

    #[test]
    fn reaction_validation_trims_catalog_entries_and_rejects_everything_else() {
        assert_eq!(validate_reaction_emoji("  👍 ").unwrap(), "👍");
        for invalid in ["", "   ", "🔥", "👍🏻", "not-an-emoji"] {
            assert_eq!(
                validate_reaction_emoji(invalid).unwrap_err(),
                DomainError::InvalidReaction
            );
        }
    }
}
