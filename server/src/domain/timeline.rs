use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

pub const MAX_TIMELINE_ENTRY_LEN: usize = 2_000;
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
            author_id: Some(author_id),
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
