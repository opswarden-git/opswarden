// --- server/src/handlers/service_connection.rs ---
//
// Thin HTTP surface over `ServiceConnectionUseCase`. Both routes live in the
// protected group (`require_auth`), so a valid JWT is mandatory. The vault is the
// only port touched; no business logic here. Secrets are write-only over HTTP:
// they go into the vault and are never returned.

use axum::{
    extract::{Extension, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::app::automation::ServiceConnectionUseCase;
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct ConnectGithubPayload {
    pub webhook_secret: String,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub connected: bool,
}

#[derive(Serialize)]
pub struct ServiceConnectionsResponse {
    pub github: ServiceStatus,
}

/// `PUT /api/service-connections/github` — store the GitHub inbound-webhook secret
/// in the encrypted vault (idempotent). Returns the connection status only; the
/// secret is never echoed back.
pub async fn connect_github(
    State(state): State<AppState>,
    Extension(_session): Extension<AuthenticatedSession>,
    Json(payload): Json<ConnectGithubPayload>,
) -> Result<Json<ServiceStatus>, DomainError> {
    ServiceConnectionUseCase::new(state.vault.clone())
        .connect_github(&payload.webhook_secret)
        .await?;
    Ok(Json(ServiceStatus { connected: true }))
}

/// `DELETE /api/service-connections/github` — remove the stored secret so the
/// integration reports disconnected. Idempotent; never returns the secret.
pub async fn disconnect_github(
    State(state): State<AppState>,
    Extension(_session): Extension<AuthenticatedSession>,
) -> Result<Json<ServiceStatus>, DomainError> {
    ServiceConnectionUseCase::new(state.vault.clone())
        .disconnect_github()
        .await?;
    Ok(Json(ServiceStatus { connected: false }))
}

/// `GET /api/service-connections` — per-service connection status. No secrets.
pub async fn list_service_connections(
    State(state): State<AppState>,
    Extension(_session): Extension<AuthenticatedSession>,
) -> Result<Json<ServiceConnectionsResponse>, DomainError> {
    let connected = ServiceConnectionUseCase::new(state.vault.clone())
        .github_connected()
        .await?;
    Ok(Json(ServiceConnectionsResponse {
        github: ServiceStatus { connected },
    }))
}
