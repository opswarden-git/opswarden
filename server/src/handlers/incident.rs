use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::incident::{
    AddTimelineEntryCommand, AddTimelineEntryResult, AddTimelineEntryUseCase,
    AssignResponderCommand, AssignResponderUseCase, ChangeIncidentStatusCommand,
    ChangeIncidentStatusUseCase, CreateIncidentCommand, CreateIncidentUseCase,
    EditTimelineEntryCommand, EditTimelineEntryResult, EditTimelineEntryUseCase,
    GetIncidentCommand, GetIncidentUseCase, IncidentActivityItem, IncidentAssigneeFilter,
    IncidentCounts, IncidentListItem, IncidentSort, ListIncidentActivityCommand,
    ListIncidentActivityUseCase, ListIncidentsCommand, ListIncidentsUseCase,
    ListTimelineEntriesCommand, ListTimelineEntriesUseCase, ReactionSummary, TimelineEntryView,
    ToggleReactionCommand, ToggleReactionUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::timeline::AVAILABLE_REACTIONS;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

/// Read-side view of an incident (list + detail), richer than the create
/// response: includes the assignee and creation time the dashboard needs.
#[derive(Serialize)]
pub struct IncidentView {
    pub incident_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub severity: String,
    pub assignee_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Incident> for IncidentView {
    fn from(incident: Incident) -> Self {
        Self {
            incident_id: incident.id,
            team_id: incident.team_id,
            title: incident.title,
            description: incident.description,
            status: incident.status.to_string(),
            severity: incident.severity.to_string(),
            assignee_id: incident.assignee,
            created_by: incident.created_by,
            created_at: incident.created_at,
            updated_at: incident.updated_at,
        }
    }
}

#[derive(Deserialize)]
pub struct ListIncidentsQuery {
    pub team_id: Uuid,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub assignee: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
}

#[derive(Serialize)]
pub struct IncidentAssigneeView {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Serialize)]
pub struct IncidentListItemView {
    pub incident_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub severity: String,
    pub assignee: Option<IncidentAssigneeView>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

impl From<IncidentListItem> for IncidentListItemView {
    fn from(item: IncidentListItem) -> Self {
        Self {
            incident_id: item.incident.id,
            team_id: item.incident.team_id,
            title: item.incident.title,
            description: item.incident.description,
            status: item.incident.status.to_string(),
            severity: item.incident.severity.to_string(),
            assignee: item.assignee.map(|member| IncidentAssigneeView {
                user_id: member.user_id,
                email: member.email,
            }),
            created_at: item.incident.created_at,
            created_by: item.incident.created_by,
            updated_at: item.incident.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct IncidentCountsView {
    pub all: u64,
    pub open: u64,
    pub acknowledged: u64,
    pub escalated: u64,
    pub resolved: u64,
}

impl From<IncidentCounts> for IncidentCountsView {
    fn from(counts: IncidentCounts) -> Self {
        Self {
            all: counts.all,
            open: counts.open,
            acknowledged: counts.acknowledged,
            escalated: counts.escalated,
            resolved: counts.resolved,
        }
    }
}

#[derive(Serialize)]
pub struct IncidentListResponse {
    pub items: Vec<IncidentListItemView>,
    pub counts: IncidentCountsView,
}

pub async fn list_incidents(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Query(query): Query<ListIncidentsQuery>,
) -> Result<Json<IncidentListResponse>, DomainError> {
    let status = query
        .status
        .as_deref()
        .map(parse_incident_status)
        .transpose()?;
    let severity = query.severity.as_deref().map(parse_severity).transpose()?;
    let assignee = match query.assignee.as_deref() {
        None | Some("") => IncidentAssigneeFilter::Any,
        Some("unassigned") => IncidentAssigneeFilter::Unassigned,
        Some(value) => IncidentAssigneeFilter::User(
            Uuid::parse_str(value).map_err(|_| DomainError::MemberNotFound)?,
        ),
    };
    let sort = match query.sort.as_deref() {
        Some("oldest") => IncidentSort::Oldest,
        Some("severity") => IncidentSort::Severity,
        _ => IncidentSort::Newest,
    };
    let use_case = ListIncidentsUseCase::new(state.teams.clone(), state.incidents.clone());
    let result = use_case
        .list_incidents(ListIncidentsCommand {
            team_id: query.team_id,
            requester_id: session.user_id,
            status,
            severity,
            assignee,
            query: query.q,
            sort,
        })
        .await?;

    Ok(Json(IncidentListResponse {
        items: result
            .items
            .into_iter()
            .map(IncidentListItemView::from)
            .collect(),
        counts: result.counts.into(),
    }))
}

pub async fn get_incident(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
) -> Result<Json<IncidentView>, DomainError> {
    let use_case = GetIncidentUseCase::new(state.teams.clone(), state.incidents.clone());
    let result = use_case
        .get_incident(GetIncidentCommand {
            incident_id,
            requester_id: session.user_id,
        })
        .await?;

    Ok(Json(IncidentView::from(result.incident)))
}

#[derive(Deserialize)]
pub struct CreateIncidentPayload {
    pub team_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
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
            description: payload.description,
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
        state.releases.clone(),
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
pub struct ReactionResponse {
    pub emoji: String,
    pub count: u64,
    pub reacted: bool,
}

#[derive(Serialize)]
pub struct TimelineEntryResponse {
    pub entry_id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author: Option<UserSummaryResponse>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub reactions: Vec<ReactionResponse>,
}

impl From<ReactionSummary> for ReactionResponse {
    fn from(reaction: ReactionSummary) -> Self {
        Self {
            emoji: reaction.emoji,
            count: reaction.count,
            reacted: reaction.reacted,
        }
    }
}

// A freshly added entry has no edit and no reactions yet.
impl From<AddTimelineEntryResult> for TimelineEntryResponse {
    fn from(result: AddTimelineEntryResult) -> Self {
        Self {
            entry_id: result.entry_id,
            incident_id: result.incident_id,
            author_id: Some(result.author_id),
            author: None,
            content: result.content,
            created_at: result.created_at,
            edited_at: None,
            reactions: Vec::new(),
        }
    }
}

// The edit response carries `edited_at`; reactions are unchanged by an edit, so
// the write path returns none and the client refetches the list for the counts.
impl From<EditTimelineEntryResult> for TimelineEntryResponse {
    fn from(result: EditTimelineEntryResult) -> Self {
        Self {
            entry_id: result.entry_id,
            incident_id: result.incident_id,
            author_id: Some(result.author_id),
            author: None,
            content: result.content,
            created_at: result.created_at,
            edited_at: result.edited_at,
            reactions: Vec::new(),
        }
    }
}

// The read view is the only path that carries aggregated reactions.
impl From<TimelineEntryView> for TimelineEntryResponse {
    fn from(view: TimelineEntryView) -> Self {
        Self {
            entry_id: view.entry.id,
            incident_id: view.entry.incident_id,
            author_id: view.entry.author_id,
            author: None,
            content: view.entry.content,
            created_at: view.entry.created_at,
            edited_at: view.entry.edited_at,
            reactions: view
                .reactions
                .into_iter()
                .map(ReactionResponse::from)
                .collect(),
        }
    }
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

    Ok((StatusCode::CREATED, Json(result.into())))
}

#[derive(Deserialize)]
pub struct EditTimelineEntryPayload {
    pub content: String,
}

pub async fn edit_timeline_entry(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((incident_id, entry_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EditTimelineEntryPayload>,
) -> Result<Json<TimelineEntryResponse>, DomainError> {
    let use_case = EditTimelineEntryUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.timeline.clone(),
        state.events.clone(),
    );
    let result = use_case
        .edit(EditTimelineEntryCommand {
            incident_id,
            entry_id,
            requester_id: session.user_id,
            content: payload.content,
        })
        .await?;

    Ok(Json(result.into()))
}

#[derive(Deserialize)]
pub struct ToggleReactionPayload {
    pub emoji: String,
}

#[derive(Serialize)]
pub struct ToggleReactionResponse {
    pub emoji: String,
    pub reacted: bool,
    pub count: u64,
}

pub async fn toggle_reaction(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((incident_id, entry_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ToggleReactionPayload>,
) -> Result<Json<ToggleReactionResponse>, DomainError> {
    let use_case = ToggleReactionUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.timeline.clone(),
        state.events.clone(),
    );
    let result = use_case
        .toggle(ToggleReactionCommand {
            incident_id,
            entry_id,
            user_id: session.user_id,
            emoji: payload.emoji,
        })
        .await?;

    Ok(Json(ToggleReactionResponse {
        emoji: result.emoji,
        reacted: result.reacted,
        count: result.count,
    }))
}

#[derive(Deserialize)]
pub struct ListTimelineEntriesQuery {
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct ListTimelineEntriesResponse {
    pub entries: Vec<TimelineEntryResponse>,
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
                reactions: reactions.into_iter().map(ReactionResponse::from).collect(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct ListIncidentActivityResponse {
    pub items: Vec<IncidentActivityItemResponse>,
}

pub async fn list_incident_activity(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Query(query): Query<ListTimelineEntriesQuery>,
) -> Result<Json<ListIncidentActivityResponse>, DomainError> {
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
        })
        .await?;

    Ok(Json(ListIncidentActivityResponse {
        items: result
            .items
            .into_iter()
            .map(IncidentActivityItemResponse::from)
            .collect(),
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

    let mut entries = Vec::with_capacity(result.entries.len());
    for view in result.entries {
        let mut response = TimelineEntryResponse::from(view);
        response.author = match response.author_id {
            Some(author_id) => {
                state
                    .users
                    .find_by_id(author_id)
                    .await?
                    .map(|user| UserSummaryResponse {
                        user_id: user.id,
                        email: user.email.as_str().to_string(),
                    })
            }
            None => None,
        };
        entries.push(response);
    }

    Ok(Json(ListTimelineEntriesResponse { entries }))
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
