// --- server/src/handlers/team/team_image.rs ---

use axum::{
    body::Body,
    extract::{Extension, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::team::{
    DeleteTeamImageCommand, GetTeamImageCommand, GetTeamImageUseCase, UpdateTeamImageCommand,
    UpdateTeamImageUseCase,
};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct UpdateTeamImagePayload {
    pub media_type: String,
    pub data_base64: String,
}

#[derive(Serialize)]
pub struct UpdateTeamImageResponse {
    pub updated_at: DateTime<Utc>,
}

pub async fn update_team_image(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<UpdateTeamImagePayload>,
) -> Result<Json<UpdateTeamImageResponse>, DomainError> {
    let content = base64::engine::general_purpose::STANDARD
        .decode(payload.data_base64)
        .map_err(|_| DomainError::InvalidTeamImage)?;
    let image = UpdateTeamImageUseCase::new(state.teams.clone())
        .update(UpdateTeamImageCommand {
            team_id,
            requester_id: session.user_id,
            media_type: payload.media_type,
            content,
        })
        .await?;
    Ok(Json(UpdateTeamImageResponse {
        updated_at: image.updated_at,
    }))
}

pub async fn delete_team_image(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<StatusCode, DomainError> {
    UpdateTeamImageUseCase::new(state.teams.clone())
        .delete(DeleteTeamImageCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Serve the stored bytes. The response is cached hard but privately: the image
/// is scoped to a team, so a shared cache must never hand it to a member of
/// another one. `nosniff` keeps a mistyped upload from being executed as
/// something else.
pub async fn get_team_image(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Response<Body>, DomainError> {
    let image = GetTeamImageUseCase::new(state.teams.clone())
        .get(GetTeamImageCommand {
            team_id,
            requester_id: session.user_id,
        })
        .await?;
    let mut response = Response::new(Body::from(image.content));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&image.media_type).map_err(|_| DomainError::Storage)?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}
