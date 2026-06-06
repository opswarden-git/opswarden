use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::team::{
    CreateTeamCommand, CreateTeamUseCase, JoinTeamCommand, JoinTeamUseCase, TransferManagerCommand,
    TransferManagerUseCase,
};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateTeamPayload {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateTeamResponse {
    pub team_id: Uuid,
    pub name: String,
    pub invitation_code: String,
}

pub async fn create_team(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<CreateTeamPayload>,
) -> Result<(StatusCode, Json<CreateTeamResponse>), DomainError> {
    let use_case = CreateTeamUseCase::new(state.teams.clone());
    let result = use_case
        .create_team(CreateTeamCommand {
            name: payload.name,
            creator_id: session.user_id,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateTeamResponse {
            team_id: result.team_id,
            name: result.name,
            invitation_code: result.invitation_code,
        }),
    ))
}

#[derive(Deserialize)]
pub struct JoinTeamPayload {
    pub invitation_code: String,
}

#[derive(Serialize)]
pub struct JoinTeamResponse {
    pub team_id: Uuid,
    pub role: String,
}

pub async fn join_team(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Json(payload): Json<JoinTeamPayload>,
) -> Result<Json<JoinTeamResponse>, DomainError> {
    let use_case = JoinTeamUseCase::new(state.teams.clone());
    let result = use_case
        .join_team(JoinTeamCommand {
            invitation_code: payload.invitation_code,
            user_id: session.user_id,
        })
        .await?;

    Ok(Json(JoinTeamResponse {
        team_id: result.team_id,
        role: result.role.to_string(),
    }))
}

#[derive(Deserialize)]
pub struct TransferManagerPayload {
    pub new_manager_id: Uuid,
}

#[derive(Serialize)]
pub struct TransferManagerResponse {
    pub team_id: Uuid,
    pub new_manager_id: Uuid,
}

pub async fn transfer_manager(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<TransferManagerPayload>,
) -> Result<Json<TransferManagerResponse>, DomainError> {
    let use_case = TransferManagerUseCase::new(state.teams.clone());
    let result = use_case
        .transfer_manager(TransferManagerCommand {
            team_id,
            requester_id: session.user_id,
            new_manager_id: payload.new_manager_id,
        })
        .await?;

    Ok(Json(TransferManagerResponse {
        team_id: result.team_id,
        new_manager_id: result.new_manager_id,
    }))
}
