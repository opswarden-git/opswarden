// --- server/src/handlers/incident/incident_reactions.rs ---

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::incident::{ToggleReactionCommand, ToggleReactionUseCase};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

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

/// Add or remove one reaction on a timeline entry. The response carries the
/// resulting count so the client renders what the server counted rather than
/// what it guessed locally.
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
