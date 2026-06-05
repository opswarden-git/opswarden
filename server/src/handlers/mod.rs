//! Inbound HTTP handlers (Axum). They translate HTTP <-> use-cases and hold no
//! business logic. At S0: liveness probe and the dynamic `/about.json`.

use axum::{extract::State, Json};
use serde::Serialize;


#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

/// Liveness probe, used by the docker-compose healthcheck.
pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
pub struct About {
    pub client: ClientInfo,
    pub server: ServerInfo,
}

#[derive(Serialize)]
pub struct ClientInfo {
    pub host: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub current_time: u64,
    pub token: String,
    pub services: Vec<ServiceCatalog>,
}

#[derive(Serialize)]
pub struct ServiceCatalog {
    pub name: String,
    pub actions: Vec<CatalogItem>,
    pub reactions: Vec<CatalogItem>,
}

#[derive(Serialize)]
pub struct CatalogItem {
    pub name: String,
    pub description: String,
}

use crate::AppState;

/// Service catalog, exposed dynamically (clients never hard-code services).
/// At S0 the catalog is empty; the rule engine (Phase 2) fills it. The
/// SHA-256 kickoff token is always present, as required by the subject.
pub async fn about(State(state): State<AppState>) -> Json<About> {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Json(About {
        client: ClientInfo {
            host: "0.0.0.0".to_string(),
        },
        server: ServerInfo {
            current_time,
            token: state.config.kickoff_token(),
            services: Vec::new(),
        },
    })
}
