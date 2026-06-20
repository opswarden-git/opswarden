// --- server/src/handlers/mod.rs ---

use axum::{extract::State, Json};
use serde::Serialize;

pub mod auth;
pub mod error;
pub mod incident;
pub mod middleware;
pub mod service_connection;
pub mod team;
pub mod webhook;
pub mod ws;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

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
            services: automation_catalog(),
        },
    })
}

/// The Action -> REAction catalog the engine actually supports, surfaced on
/// `/about.json` so the contract is server-driven (nothing hard-coded client
/// side). Grows as services/Actions/REActions are added in `adapters/webhook`
/// and the rule engine.
fn automation_catalog() -> Vec<ServiceCatalog> {
    vec![ServiceCatalog {
        name: "github".to_string(),
        actions: vec![CatalogItem {
            name: "ci_failed".to_string(),
            description: "A GitHub Actions workflow run completed with a failing conclusion"
                .to_string(),
        }],
        reactions: vec![
            CatalogItem {
                name: "create_incident".to_string(),
                description: "Open a high-severity incident in the configured team".to_string(),
            },
            CatalogItem {
                name: "notify".to_string(),
                description:
                    "Send a notification to a configured HTTP webhook (Slack, Discord, or any URL)"
                        .to_string(),
            },
        ],
    }]
}
