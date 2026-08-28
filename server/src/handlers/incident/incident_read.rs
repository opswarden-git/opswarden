// --- server/src/handlers/incident/incident_read.rs ---

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::incident::{MarkIncidentReadCommand, MarkIncidentReadUseCase};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct MarkIncidentReadPayload {
    pub read_through: DateTime<Utc>,
}

/// Move a member's read position on one incident. The position belongs to the
/// server rather than the browser, so unread state survives a refresh and
/// follows the member from the web client to the desktop shell.
pub async fn mark_incident_read(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(incident_id): Path<Uuid>,
    Json(payload): Json<MarkIncidentReadPayload>,
) -> Result<StatusCode, DomainError> {
    MarkIncidentReadUseCase::new(state.teams.clone(), state.incidents.clone())
        .mark(MarkIncidentReadCommand {
            incident_id,
            requester_id: session.user_id,
            read_through: payload.read_through,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
