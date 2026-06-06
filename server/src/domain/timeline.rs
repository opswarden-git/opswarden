use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

pub const MAX_TIMELINE_ENTRY_LEN: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl TimelineEntry {
    pub fn new(
        incident_id: Uuid,
        author_id: Uuid,
        content: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let content = content.into();
        let trimmed = content.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_TIMELINE_ENTRY_LEN {
            return Err(DomainError::InvalidTimelineEntry);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            incident_id,
            author_id,
            content: trimmed.to_string(),
            created_at: Utc::now(),
        })
    }
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
}
