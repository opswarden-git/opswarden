use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::channel::Channel;
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::team::Role;
use crate::handlers::middleware::AuthenticatedSession;
use crate::ports::EventPublisher;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct ChannelResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub created_at: String,
}

impl From<Channel> for ChannelResponse {
    fn from(channel: Channel) -> Self {
        Self {
            id: channel.id,
            team_id: channel.team_id,
            name: channel.name,
            created_at: channel.created_at.to_rfc3339(),
        }
    }
}

async fn require_responder(state: &AppState, team_id: Uuid, user_id: Uuid) -> Result<Role, DomainError> {
    let role = state.teams.find_member_role(team_id, user_id).await?;
    match role {
        Some(r) if r == Role::Manager || r == Role::Responder => Ok(r),
        _ => Err(DomainError::Forbidden),
    }
}

pub async fn list_channels(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<ChannelResponse>>, DomainError> {
    require_responder(&state, team_id, session.user_id).await?;

    let channels = state.channels.list_channels_for_team(team_id).await?;
    let responses: Vec<ChannelResponse> = channels.into_iter().map(ChannelResponse::from).collect();
    Ok(Json(responses))
}

pub async fn create_channel(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), DomainError> {
    require_responder(&state, team_id, session.user_id).await?;

    let channel_name = payload.name.trim();
    if channel_name.is_empty() {
        return Err(DomainError::InvalidChannelName);
    }

    let channel = Channel {
        id: Uuid::new_v4(),
        team_id,
        name: channel_name.to_string(),
        created_at: Utc::now(),
    };

    state.channels.create_channel(&channel).await?;

    state.events.publish(DomainEvent::ChannelCreated {
        team_id,
        channel_id: channel.id,
        name: channel.name.clone(),
        by: session.user_id,
    }).await;

    Ok((StatusCode::CREATED, Json(ChannelResponse::from(channel))))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, channel_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    require_responder(&state, team_id, session.user_id).await?;

    state.channels.delete_channel(team_id, channel_id).await?;

    state.events.publish(DomainEvent::ChannelDeleted {
        team_id,
        channel_id,
        by: session.user_id,
    }).await;

    Ok(StatusCode::NO_CONTENT)
}
