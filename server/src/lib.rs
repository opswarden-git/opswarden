// --- server/src/lib.rs ---

#![forbid(unsafe_code)]

pub mod adapters;
pub mod app;
pub mod config;
pub mod domain;
pub mod handlers;
pub mod ports;

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::adapters::ws::WsHub;
use crate::ports::{
    Clock, IncidentRepo, PasswordHasher, TeamRepo, TimelineRepo, TokenRevocationRepo, TokenService,
    UserRepo,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<dyn UserRepo + Send + Sync>,
    pub teams: Arc<dyn TeamRepo + Send + Sync>,
    pub incidents: Arc<dyn IncidentRepo + Send + Sync>,
    pub timeline: Arc<dyn TimelineRepo + Send + Sync>,
    pub hasher: Arc<dyn PasswordHasher + Send + Sync>,
    pub tokens: Arc<dyn TokenService + Send + Sync>,
    pub token_revocations: Arc<dyn TokenRevocationRepo + Send + Sync>,
    /// Concrete WebSocket hub: used as `dyn EventPublisher` by the use cases and
    /// directly by the `/ws` handler to register/unregister connections.
    pub events: Arc<WsHub>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub config: config::Config,
}

pub fn build_app(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/me", get(handlers::auth::get_me))
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/teams", post(handlers::team::create_team))
        .route("/api/teams/join", post(handlers::team::join_team))
        .route(
            "/api/teams/{team_id}/manager",
            put(handlers::team::transfer_manager),
        )
        .route("/api/incidents", post(handlers::incident::create_incident))
        .route(
            "/api/incidents/{incident_id}/status",
            put(handlers::incident::change_status),
        )
        .route(
            "/api/incidents/{incident_id}/assign",
            put(handlers::incident::assign_responder),
        )
        .route(
            "/api/incidents/{incident_id}/timeline",
            post(handlers::incident::add_timeline_entry)
                .get(handlers::incident::list_timeline_entries),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::middleware::require_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health))
        .route("/about.json", get(handlers::about))
        .route("/api/auth/sign-up", post(handlers::auth::sign_up))
        .route("/api/auth/sign-in", post(handlers::auth::sign_in))
        // Public upgrade: the WS authenticates in-band via its first message.
        .route("/ws", get(handlers::ws::ws_handler))
        .merge(protected_routes)
        .with_state(state)
}
