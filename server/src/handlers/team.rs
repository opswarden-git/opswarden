use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::team::{
    BanMemberCommand, BanMemberUseCase, BanRequest, CreateTeamCommand, CreateTeamUseCase,
    JoinTeamCommand, JoinTeamUseCase, KickMemberCommand, KickMemberUseCase, ListBansCommand,
    ListBansUseCase, ListTeamMembersCommand, ListTeamMembersUseCase, ListTeamsCommand,
    ListTeamsUseCase, SetMemberRoleCommand, SetMemberRoleUseCase, TransferManagerCommand,
    TransferManagerUseCase,
};
use crate::domain::error::DomainError;
use crate::domain::team::{BanKind, Role};
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Serialize)]
pub struct TeamSummaryResponse {
    pub team_id: Uuid,
    pub name: String,
    pub invitation_code: String,
    pub role: String,
}

pub async fn list_teams(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
) -> Result<Json<Vec<TeamSummaryResponse>>, DomainError> {
    let use_case = ListTeamsUseCase::new(state.teams.clone());
    let result = use_case
        .list_teams(ListTeamsCommand {
            user_id: session.user_id,
        })
        .await?;

    Ok(Json(
        result
            .teams
            .into_iter()
            .map(|team| TeamSummaryResponse {
                team_id: team.team_id,
                name: team.name,
                invitation_code: team.invitation_code,
                role: team.role.to_string(),
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct TeamMemberResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

pub async fn list_members(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<TeamMemberResponse>>, DomainError> {
    let use_case = ListTeamMembersUseCase::new(state.teams.clone());
    let result = use_case
        .list_members(ListTeamMembersCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;

    Ok(Json(
        result
            .members
            .into_iter()
            .map(|member| TeamMemberResponse {
                user_id: member.user_id,
                email: member.email,
                role: member.role.to_string(),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetMemberRolePayload {
    pub role: String,
}

/// Only Observer and Responder are settable here; "manager" (and anything else)
/// is rejected — the Manager seat moves through `transfer_manager`, not this route.
fn parse_assignable_role(value: &str) -> Result<Role, DomainError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "observer" => Ok(Role::Observer),
        "responder" => Ok(Role::Responder),
        _ => Err(DomainError::InvalidRole),
    }
}

pub async fn set_member_role(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SetMemberRolePayload>,
) -> Result<StatusCode, DomainError> {
    let new_role = parse_assignable_role(&payload.role)?;
    let use_case = SetMemberRoleUseCase::new(state.teams.clone());
    use_case
        .set_member_role(SetMemberRoleCommand {
            team_id,
            requester_id: session.user_id,
            target_user_id: user_id,
            new_role,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

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

pub async fn delete_team(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<StatusCode, DomainError> {
    let use_case = crate::app::team::DeleteTeamUseCase::new(state.teams.clone());
    use_case
        .delete_team(crate::app::team::DeleteTeamCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn leave_team(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<StatusCode, DomainError> {
    let use_case = crate::app::team::LeaveTeamUseCase::new(state.teams.clone());
    use_case
        .leave_team(crate::app::team::LeaveTeamCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Kick a member: `DELETE /api/teams/{team_id}/members/{user_id}`. Manager-only;
/// removes membership without recording a ban.
pub async fn kick_member(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    let use_case = KickMemberUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.events.clone(),
    );
    use_case
        .kick_member(KickMemberCommand {
            team_id,
            requester_id: session.user_id,
            target_user_id: user_id,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct BanMemberPayload {
    pub user_id: Uuid,
    /// "temporary" (requires `expires_at`) or "permanent".
    pub kind: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct BanMemberResponse {
    pub user_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub removed_membership: bool,
}

/// Ban a user: `POST /api/teams/{team_id}/bans`. Manager-only; records the ban
/// and drops the target's membership if they were a member.
pub async fn ban_member(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<BanMemberPayload>,
) -> Result<(StatusCode, Json<BanMemberResponse>), DomainError> {
    let request = match payload.kind.trim().to_ascii_lowercase().as_str() {
        "permanent" => BanRequest::Permanent,
        "temporary" => BanRequest::Temporary {
            expires_at: payload.expires_at.ok_or(DomainError::InvalidBanExpiry)?,
        },
        _ => return Err(DomainError::InvalidBanKind),
    };

    let use_case = BanMemberUseCase::new(
        state.teams.clone(),
        state.incidents.clone(),
        state.users.clone(),
        state.events.clone(),
    );
    let result = use_case
        .ban_member(BanMemberCommand {
            team_id,
            requester_id: session.user_id,
            target_user_id: payload.user_id,
            request,
            reason: payload.reason,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(BanMemberResponse {
            user_id: result.user_id,
            expires_at: result.expires_at,
            removed_membership: result.removed_membership,
        }),
    ))
}

#[derive(Serialize)]
pub struct BanResponse {
    pub user_id: Uuid,
    pub kind: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

/// List a team's bans: `GET /api/teams/{team_id}/bans`. Manager-only.
pub async fn list_bans(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<BanResponse>>, DomainError> {
    let use_case = ListBansUseCase::new(state.teams.clone());
    let result = use_case
        .list_bans(ListBansCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;

    let now = Utc::now();
    Ok(Json(
        result
            .bans
            .into_iter()
            .map(|ban| {
                let active = ban.is_active(now);
                let expires_at = ban.expires_at();
                let kind = match &ban.kind {
                    BanKind::Temporary { .. } => "temporary",
                    BanKind::Permanent => "permanent",
                }
                .to_string();
                BanResponse {
                    user_id: ban.user_id,
                    kind,
                    expires_at,
                    reason: ban.reason,
                    created_by: ban.created_by,
                    created_at: ban.created_at,
                    active,
                }
            })
            .collect(),
    ))
}
