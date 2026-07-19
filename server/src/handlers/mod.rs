// --- server/src/handlers/mod.rs ---

use axum::{extract::State, Json};
use serde::Serialize;

pub mod auth;
pub mod error;
pub mod gif;
pub mod incident;
pub mod middleware;
pub mod private_message;
pub mod release;
pub mod team;
pub mod team_automation;
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
    pub label: String,
    pub actions: Vec<CatalogItem>,
    pub reactions: Vec<CatalogItem>,
}

#[derive(Serialize)]
pub struct CatalogItem {
    pub name: String,
    pub label: String,
    pub description: String,
    pub connection_service: Option<String>,
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
    crate::domain::automation_catalog::AUTOMATION_CATALOG
        .iter()
        .map(|service| ServiceCatalog {
            name: service.service.to_string(),
            label: service.label.to_string(),
            actions: service.actions.iter().map(catalog_item).collect(),
            reactions: service.reactions.iter().map(catalog_item).collect(),
        })
        .collect()
}

fn catalog_item(item: &crate::domain::automation_catalog::CatalogCapability) -> CatalogItem {
    CatalogItem {
        name: item.kind.to_string(),
        label: item.label.to_string(),
        description: item.description.to_string(),
        connection_service: item.connection_service.map(str::to_string),
    }
}
