use serde::Serialize;
use uuid::Uuid;

pub const MAX_MESSAGE_LEN: usize = 2_000;
pub const MAX_MESSAGE_ATTACHMENTS: usize = 4;
pub const MAX_MESSAGE_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_MESSAGE_ATTACHMENTS_TOTAL_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAttachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub file_name: String,
    pub media_type: String,
    pub size_bytes: usize,
    pub content: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReactionSummary {
    pub emoji: String,
    pub count: u64,
    pub reacted: bool,
}

pub struct ValidatedAttachment {
    pub file_name: String,
    pub media_type: String,
    pub content: Vec<u8>,
}

pub fn normalize_message_content(content: impl Into<String>, allow_empty: bool) -> Option<String> {
    let content = content.into();
    let trimmed = content.trim();
    (trimmed.len() <= MAX_MESSAGE_LEN && (allow_empty || !trimmed.is_empty()))
        .then(|| trimmed.to_string())
}

pub fn validate_attachments(
    attachments: Vec<(String, String, Vec<u8>)>,
) -> Option<Vec<ValidatedAttachment>> {
    if attachments.len() > MAX_MESSAGE_ATTACHMENTS
        || attachments
            .iter()
            .map(|(_, _, content)| content.len())
            .sum::<usize>()
            > MAX_MESSAGE_ATTACHMENTS_TOTAL_BYTES
    {
        return None;
    }
    attachments
        .into_iter()
        .map(|(file_name, media_type, content)| {
            let file_name = file_name.trim();
            let media_type = media_type.trim().to_ascii_lowercase();
            (!file_name.is_empty()
                && file_name.len() <= 255
                && !media_type.is_empty()
                && media_type.len() <= 127
                && !content.is_empty()
                && content.len() <= MAX_MESSAGE_ATTACHMENT_BYTES
                && allowed_media_type(&media_type))
            .then(|| ValidatedAttachment {
                file_name: file_name.to_string(),
                media_type,
                content,
            })
        })
        .collect()
}

fn allowed_media_type(media_type: &str) -> bool {
    matches!(
        media_type,
        "image/jpeg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "application/pdf"
            | "application/json"
            | "application/zip"
            | "application/gzip"
            | "application/octet-stream"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "text/plain"
            | "text/csv"
            | "text/markdown"
            | "text/yaml"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationScope {
    Direct { first_user: Uuid, second_user: Uuid },
    Incident { team_id: Uuid, incident_id: Uuid },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationFeature {
    SendText,
    SendGif,
    EditOwnMessage,
    React,
    AttachFiles,
    PaginatedHistory,
    Presence,
    Typing,
    CollaborativeCursors,
    SystemEvents,
}

const DIRECT_FEATURES: [ConversationFeature; 7] = [
    ConversationFeature::SendText,
    ConversationFeature::SendGif,
    ConversationFeature::EditOwnMessage,
    ConversationFeature::AttachFiles,
    ConversationFeature::PaginatedHistory,
    ConversationFeature::Presence,
    ConversationFeature::Typing,
];

const INCIDENT_FEATURES: [ConversationFeature; 10] = [
    ConversationFeature::SendText,
    ConversationFeature::SendGif,
    ConversationFeature::EditOwnMessage,
    ConversationFeature::React,
    ConversationFeature::AttachFiles,
    ConversationFeature::PaginatedHistory,
    ConversationFeature::Presence,
    ConversationFeature::Typing,
    ConversationFeature::CollaborativeCursors,
    ConversationFeature::SystemEvents,
];

impl ConversationScope {
    pub fn features(self) -> &'static [ConversationFeature] {
        match self {
            Self::Direct { .. } => &DIRECT_FEATURES,
            Self::Incident { .. } => &INCIDENT_FEATURES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_and_incident_capabilities_are_explicit() {
        let direct = ConversationScope::Direct {
            first_user: Uuid::new_v4(),
            second_user: Uuid::new_v4(),
        };
        let incident = ConversationScope::Incident {
            team_id: Uuid::new_v4(),
            incident_id: Uuid::new_v4(),
        };

        assert!(direct
            .features()
            .contains(&ConversationFeature::AttachFiles));
        assert!(!direct.features().contains(&ConversationFeature::React));
        assert!(!direct
            .features()
            .contains(&ConversationFeature::SystemEvents));
        assert!(incident
            .features()
            .contains(&ConversationFeature::CollaborativeCursors));
        assert!(incident
            .features()
            .contains(&ConversationFeature::AttachFiles));
        assert!(incident.features().contains(&ConversationFeature::React));
    }

    #[test]
    fn shared_message_policy_normalizes_text_and_bounds_files() {
        assert_eq!(
            normalize_message_content("  hello  ", false).as_deref(),
            Some("hello")
        );
        assert!(normalize_message_content("   ", false).is_none());
        assert!(normalize_message_content("   ", true).is_some());
        assert!(validate_attachments(vec![(
            "runbook.pdf".into(),
            "application/pdf".into(),
            vec![1, 2, 3],
        )])
        .is_some());
        assert!(
            validate_attachments(vec![("payload.html".into(), "text/html".into(), vec![1],)])
                .is_none()
        );
    }
}
