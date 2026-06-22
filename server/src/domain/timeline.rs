use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

pub const MAX_TIMELINE_ENTRY_LEN: usize = 2_000;
/// Generous enough for multi-codepoint emoji (ZWJ sequences, skin tones), tight
/// enough to reject pasted text masquerading as a reaction.
pub const MAX_REACTION_EMOJI_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /// `Some` once the entry has been edited; `created_at` is never moved.
    pub edited_at: Option<DateTime<Utc>>,
}

impl TimelineEntry {
    pub fn new(
        incident_id: Uuid,
        author_id: Uuid,
        content: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let content = Self::validate_content(content)?;
        Ok(Self {
            id: Uuid::new_v4(),
            incident_id,
            author_id,
            content,
            created_at: Utc::now(),
            edited_at: None,
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
        let content = content.into();
        let trimmed = content.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_TIMELINE_ENTRY_LEN {
            return Err(DomainError::InvalidTimelineEntry);
        }
        Ok(trimmed.to_string())
    }
}

/// Pure validation of a reaction emoji: non-blank, bounded length. Returns the
/// trimmed emoji to store.
pub fn validate_reaction_emoji(emoji: &str) -> Result<String, DomainError> {
    let trimmed = emoji.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_REACTION_EMOJI_LEN {
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
    fn reaction_emoji_validation() {
        assert_eq!(validate_reaction_emoji("  👍 ").unwrap(), "👍");
        assert_eq!(
            validate_reaction_emoji("   ").unwrap_err(),
            DomainError::InvalidReaction
        );
        assert_eq!(
            validate_reaction_emoji(&"x".repeat(MAX_REACTION_EMOJI_LEN + 1)).unwrap_err(),
            DomainError::InvalidReaction
        );
    }
}
