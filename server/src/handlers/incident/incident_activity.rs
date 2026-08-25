use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{Response, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::incident::{
    GetTimelineAttachmentUseCase, IncidentActivityItem, ListIncidentActivityCommand,
    ListIncidentActivityUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::timeline::AVAILABLE_REACTIONS;
use crate::domain::{ConversationFeature, ConversationScope};
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

use crate::handlers::conversation::{
    attachment_download_response, AttachmentResponse, ConversationCursorResponse, ReactionResponse,
};

#[derive(Deserialize)]
pub struct ListIncidentActivityQuery {
    pub limit: Option<u32>,
    pub before_created_at: Option<DateTime<Utc>>,
    pub before_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct UserSummaryResponse {
    pub user_id: Uuid,
    pub email: String,
}

impl From<crate::domain::user::UserSummary> for UserSummaryResponse {
    fn from(summary: crate::domain::user::UserSummary) -> Self {
        Self {
            user_id: summary.user_id,
            email: summary.email,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncidentActivityItemResponse {
    SystemEvent {
        id: Uuid,
        kind: String,
        actor: Option<UserSummaryResponse>,
        subject: Option<UserSummaryResponse>,
        data: serde_json::Value,
        created_at: DateTime<Utc>,
    },
    HumanNote {
        entry_id: Uuid,
        author: Option<UserSummaryResponse>,
        content: String,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
        attachments: Vec<AttachmentResponse>,
        reactions: Vec<ReactionResponse>,
    },
}

impl From<IncidentActivityItem> for IncidentActivityItemResponse {
    fn from(item: IncidentActivityItem) -> Self {
        match item {
            IncidentActivityItem::System {
                event,
                actor,
                subject,
            } => Self::SystemEvent {
                id: event.id,
                kind: event.kind.to_string(),
                actor: actor.map(UserSummaryResponse::from),
                subject: subject.map(UserSummaryResponse::from),
                data: event.data,
                created_at: event.created_at,
            },
            IncidentActivityItem::Note {
                entry,
                author,
                reactions,
            } => Self::HumanNote {
                entry_id: entry.id,
                author: author.map(UserSummaryResponse::from),
                content: entry.content,
                created_at: entry.created_at,
                edited_at: entry.edited_at,
                attachments: entry.attachments.into_iter().map(Into::into).collect(),
                reactions: reactions.into_iter().map(ReactionResponse::from).collect(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct ListIncidentActivityResponse {
    pub items: Vec<IncidentActivityItemResponse>,
    pub next_cursor: Option<ConversationCursorResponse>,
    pub features: Vec<ConversationFeature>,
}

pub async fn list_incident_activity(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Query(query): Query<ListIncidentActivityQuery>,
) -> Result<Json<ListIncidentActivityResponse>, DomainError> {
    // Half a cursor is a client bug, not a first page: refuse it rather than
    // silently rewinding the reader to the top of the incident.
    let before = match (query.before_created_at, query.before_id) {
        (Some(at), Some(id)) => Some((at, id)),
        (None, None) => None,
        _ => return Err(DomainError::InvalidTimelineEntry),
    };
    let use_case = ListIncidentActivityUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.timeline.clone(),
        state.users.clone(),
    );
    let result = use_case
        .list(ListIncidentActivityCommand {
            incident_id,
            requester_id: session.user_id,
            limit: query.limit,
            before,
        })
        .await?;

    Ok(Json(ListIncidentActivityResponse {
        items: result
            .items
            .into_iter()
            .map(IncidentActivityItemResponse::from)
            .collect(),
        next_cursor: result
            .next_cursor
            .map(|(created_at, id)| ConversationCursorResponse { created_at, id }),
        features: ConversationScope::Incident {
            team_id: result.team_id,
            incident_id,
        }
        .features()
        .to_vec(),
    }))
}

#[derive(Serialize)]
pub struct AvailableReactionsResponse {
    pub reactions: Vec<&'static str>,
}

pub async fn available_reactions() -> Json<AvailableReactionsResponse> {
    Json(AvailableReactionsResponse {
        reactions: AVAILABLE_REACTIONS.to_vec(),
    })
}

pub async fn download_timeline_attachment(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(attachment_id): Path<Uuid>,
) -> Result<Response<Body>, DomainError> {
    let attachment = GetTimelineAttachmentUseCase::new(state.timeline.clone())
        .get(attachment_id, session.user_id)
        .await?;
    Ok(attachment_download_response(attachment))
}

pub async fn delete_incident(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
) -> Result<StatusCode, DomainError> {
    let use_case = crate::app::incident::DeleteIncidentUseCase::new(
        state.incidents.clone(),
        state.teams.clone(),
    );
    use_case
        .delete_incident(crate::app::incident::DeleteIncidentCommand {
            incident_id,
            requester_id: session.user_id,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub(super) fn parse_severity(value: &str) -> Result<Severity, DomainError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(DomainError::InvalidSeverity),
    }
}

pub(super) fn parse_incident_status(value: &str) -> Result<IncidentStatus, DomainError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" => Ok(IncidentStatus::Open),
        "acknowledged" => Ok(IncidentStatus::Acknowledged),
        "escalated" => Ok(IncidentStatus::Escalated),
        "resolved" => Ok(IncidentStatus::Resolved),
        _ => Err(DomainError::InvalidIncidentStatus),
    }
}
