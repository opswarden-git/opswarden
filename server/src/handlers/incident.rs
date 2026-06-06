use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::incident::{
    AddTimelineEntryCommand, AddTimelineEntryUseCase, AssignResponderCommand,
    AssignResponderUseCase, ChangeIncidentStatusCommand, ChangeIncidentStatusUseCase,
    CreateIncidentCommand, CreateIncidentUseCase, ListTimelineEntriesCommand,
    ListTimelineEntriesUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateIncidentPayload {
    pub team_id: Uuid,
    pub title: String,
    pub severity: String,
}

#[derive(Serialize)]
pub struct IncidentResponse {
    pub incident_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub status: String,
    pub severity: String,
}

pub async fn create_incident(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<CreateIncidentPayload>,
) -> Result<(StatusCode, Json<IncidentResponse>), DomainError> {
    let use_case = CreateIncidentUseCase::new(state.teams.clone(), state.incidents.clone());
    let result = use_case
        .create_incident(CreateIncidentCommand {
            team_id: payload.team_id,
            requester_id: session.user_id,
            title: payload.title,
            severity: parse_severity(&payload.severity)?,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(IncidentResponse {
            incident_id: result.incident_id,
            team_id: result.team_id,
            title: result.title,
            status: result.status.to_string(),
            severity: result.severity.to_string(),
        }),
    ))
}

#[derive(Deserialize)]
pub struct ChangeIncidentStatusPayload {
    pub status: String,
}

#[derive(Serialize)]
pub struct ChangeIncidentStatusResponse {
    pub incident_id: Uuid,
    pub status: String,
    pub severity: String,
    pub changed: bool,
}

pub async fn change_status(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Json(payload): Json<ChangeIncidentStatusPayload>,
) -> Result<Json<ChangeIncidentStatusResponse>, DomainError> {
    let use_case = ChangeIncidentStatusUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.events.clone(),
    );
    let result = use_case
        .change_status(ChangeIncidentStatusCommand {
            incident_id,
            requester_id: session.user_id,
            new_status: parse_incident_status(&payload.status)?,
        })
        .await?;

    Ok(Json(ChangeIncidentStatusResponse {
        incident_id: result.incident_id,
        status: result.status.to_string(),
        severity: result.severity.to_string(),
        changed: result.changed,
    }))
}

#[derive(Deserialize)]
pub struct AssignResponderPayload {
    pub assignee_id: Uuid,
}

#[derive(Serialize)]
pub struct AssignResponderResponse {
    pub incident_id: Uuid,
    pub assignee_id: Uuid,
    pub changed: bool,
}

pub async fn assign_responder(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Json(payload): Json<AssignResponderPayload>,
) -> Result<Json<AssignResponderResponse>, DomainError> {
    let use_case = AssignResponderUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.events.clone(),
    );
    let result = use_case
        .assign(AssignResponderCommand {
            incident_id,
            requester_id: session.user_id,
            assignee_id: payload.assignee_id,
        })
        .await?;

    Ok(Json(AssignResponderResponse {
        incident_id: result.incident_id,
        assignee_id: result.assignee_id,
        changed: result.changed,
    }))
}

#[derive(Deserialize)]
pub struct AddTimelineEntryPayload {
    pub content: String,
}

#[derive(Serialize)]
pub struct TimelineEntryResponse {
    pub entry_id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub async fn add_timeline_entry(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Json(payload): Json<AddTimelineEntryPayload>,
) -> Result<(StatusCode, Json<TimelineEntryResponse>), DomainError> {
    let use_case = AddTimelineEntryUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.timeline.clone(),
        state.events.clone(),
    );
    let result = use_case
        .add_timeline_entry(AddTimelineEntryCommand {
            incident_id,
            author_id: session.user_id,
            content: payload.content,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(TimelineEntryResponse {
            entry_id: result.entry_id,
            incident_id: result.incident_id,
            author_id: result.author_id,
            content: result.content,
            created_at: result.created_at,
        }),
    ))
}

#[derive(Deserialize)]
pub struct ListTimelineEntriesQuery {
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ListTimelineEntriesResponse {
    pub entries: Vec<TimelineEntryResponse>,
}

pub async fn list_timeline_entries(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Query(query): Query<ListTimelineEntriesQuery>,
) -> Result<Json<ListTimelineEntriesResponse>, DomainError> {
    let use_case = ListTimelineEntriesUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.timeline.clone(),
    );
    let result = use_case
        .list_entries(ListTimelineEntriesCommand {
            incident_id,
            requester_id: session.user_id,
            limit: query.limit,
        })
        .await?;

    Ok(Json(ListTimelineEntriesResponse {
        entries: result
            .entries
            .into_iter()
            .map(|entry| TimelineEntryResponse {
                entry_id: entry.id,
                incident_id: entry.incident_id,
                author_id: entry.author_id,
                content: entry.content,
                created_at: entry.created_at,
            })
            .collect(),
    }))
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

fn parse_severity(value: &str) -> Result<Severity, DomainError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(DomainError::InvalidSeverity),
    }
}

fn parse_incident_status(value: &str) -> Result<IncidentStatus, DomainError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" => Ok(IncidentStatus::Open),
        "acknowledged" => Ok(IncidentStatus::Acknowledged),
        "escalated" => Ok(IncidentStatus::Escalated),
        "resolved" => Ok(IncidentStatus::Resolved),
        _ => Err(DomainError::InvalidIncidentStatus),
    }
}
