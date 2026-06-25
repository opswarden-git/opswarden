// --- server/src/handlers/release.rs ---
//
// HTTP surface for releases, all routes behind `require_auth`. Handlers stay thin
// translators: authorization, the state machine and the blocking recompute all
// live in the release use-cases.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::release::{
    CancelReleaseCommand, CancelReleaseUseCase, CreateReleaseCommand, CreateReleaseUseCase,
    GetReleaseCommand, GetReleaseUseCase, LinkIncidentCommand, LinkIncidentUseCase,
    ListReleasesCommand, ListReleasesUseCase, ReleaseDetail, UnlinkIncidentCommand,
    UnlinkIncidentUseCase, ValidateReleaseStepCommand, ValidateReleaseStepUseCase,
};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Serialize)]
pub struct ReleaseStepView {
    pub name: String,
    pub validated: bool,
    pub validated_by: Option<Uuid>,
    pub validated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct ReleaseView {
    pub release_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    /// Effective state, with `blocked` already resolved from linked incidents.
    pub state: String,
    pub steps: Vec<ReleaseStepView>,
    pub linked_incident_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<ReleaseDetail> for ReleaseView {
    fn from(detail: ReleaseDetail) -> Self {
        let ReleaseDetail {
            release,
            effective_state,
            linked_incident_ids,
        } = detail;
        Self {
            release_id: release.id,
            team_id: release.team_id,
            title: release.title,
            state: effective_state.to_string(),
            steps: release
                .steps
                .into_iter()
                .map(|step| ReleaseStepView {
                    name: step.name,
                    validated: step.validated_at.is_some(),
                    validated_by: step.validated_by,
                    validated_at: step.validated_at,
                })
                .collect(),
            linked_incident_ids,
            created_at: release.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateReleasePayload {
    pub team_id: Uuid,
    pub title: String,
    pub steps: Vec<String>,
}

pub async fn create_release(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<CreateReleasePayload>,
) -> Result<(StatusCode, Json<ReleaseView>), DomainError> {
    let use_case = CreateReleaseUseCase::new(state.teams.clone(), state.releases.clone());
    let detail = use_case
        .create(CreateReleaseCommand {
            team_id: payload.team_id,
            title: payload.title,
            steps: payload.steps,
            requester_id: session.user_id,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(detail.into())))
}

#[derive(Deserialize)]
pub struct ListReleasesQuery {
    pub team_id: Uuid,
}

pub async fn list_releases(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Query(query): Query<ListReleasesQuery>,
) -> Result<Json<Vec<ReleaseView>>, DomainError> {
    let use_case = ListReleasesUseCase::new(state.teams.clone(), state.releases.clone());
    let releases = use_case
        .list(ListReleasesCommand {
            team_id: query.team_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(releases.into_iter().map(ReleaseView::from).collect()))
}

pub async fn get_release(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(release_id): Path<Uuid>,
) -> Result<Json<ReleaseView>, DomainError> {
    let use_case = GetReleaseUseCase::new(state.teams.clone(), state.releases.clone());
    let detail = use_case
        .get(GetReleaseCommand {
            release_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(detail.into()))
}

pub async fn validate_release_step(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((release_id, step)): Path<(Uuid, String)>,
) -> Result<Json<ReleaseView>, DomainError> {
    let use_case = ValidateReleaseStepUseCase::new(
        state.teams.clone(),
        state.releases.clone(),
        state.events.clone(),
    );
    let detail = use_case
        .validate(ValidateReleaseStepCommand {
            release_id,
            step,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(detail.into()))
}

pub async fn link_incident(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((release_id, incident_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReleaseView>, DomainError> {
    let use_case = LinkIncidentUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.releases.clone(),
        state.events.clone(),
    );
    let detail = use_case
        .link(LinkIncidentCommand {
            release_id,
            incident_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(detail.into()))
}

pub async fn unlink_incident(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((release_id, incident_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReleaseView>, DomainError> {
    let use_case = UnlinkIncidentUseCase::new(
        state.teams.clone(),
        state.releases.clone(),
        state.events.clone(),
    );
    let detail = use_case
        .unlink(UnlinkIncidentCommand {
            release_id,
            incident_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(detail.into()))
}

pub async fn cancel_release(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(release_id): Path<Uuid>,
) -> Result<Json<ReleaseView>, DomainError> {
    let use_case = CancelReleaseUseCase::new(
        state.teams.clone(),
        state.releases.clone(),
        state.events.clone(),
    );
    let detail = use_case
        .cancel(CancelReleaseCommand {
            release_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(Json(detail.into()))
}
