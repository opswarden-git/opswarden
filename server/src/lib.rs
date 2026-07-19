// --- server/src/lib.rs ---

#![forbid(unsafe_code)]

pub mod adapters;
pub mod app;
pub mod config;
pub mod domain;
pub mod handlers;
pub mod ports;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
    Router,
};

use crate::adapters::ws::WsHub;
use crate::ports::{
    AutomationRuleRepo, AutomationRunRepo, Clock, ConnectionCredentialVault, GifSearch,
    IncidentRepo, Notifier, OAuthClient, PasswordHasher, PrivateMessageRepo, ReleaseRepo,
    ServiceConnectionRepo, TeamRepo, TimelineRepo, TokenRevocationRepo, TokenService, UserRepo,
    WebhookDeliveryRepo, WebhookParser, WebhookVerifier,
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
    pub oauth: Arc<dyn OAuthClient + Send + Sync>,
    pub token_revocations: Arc<dyn TokenRevocationRepo + Send + Sync>,
    /// Concrete WebSocket hub: used as `dyn EventPublisher` by the use cases and
    /// directly by the `/ws` handler to register/unregister connections.
    pub events: Arc<WsHub>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub webhook_verifier: Arc<dyn WebhookVerifier + Send + Sync>,
    pub webhook_parser: Arc<dyn WebhookParser + Send + Sync>,
    /// Team-scoped automation resources. Connections own every credential and
    /// every inbound webhook resolves through an opaque connection id.
    pub service_connections: Arc<dyn ServiceConnectionRepo + Send + Sync>,
    pub connection_credentials: Arc<dyn ConnectionCredentialVault + Send + Sync>,
    pub automation_rules: Arc<dyn AutomationRuleRepo + Send + Sync>,
    pub webhook_deliveries: Arc<dyn WebhookDeliveryRepo + Send + Sync>,
    pub automation_runs: Arc<dyn AutomationRunRepo + Send + Sync>,
    pub notifier: Arc<dyn Notifier + Send + Sync>,
    /// External GIF search (GIPHY) for timeline GIFs (RTC2 web_api_integration).
    pub gifs: Arc<dyn GifSearch + Send + Sync>,
    /// Bilateral 1-to-1 direct messages between team-sharing users (RTC2 web_pm).
    pub private_messages: Arc<dyn PrivateMessageRepo + Send + Sync>,
    /// Releases with sequential steps and incident-driven blocking (VIGIL P1).
    pub releases: Arc<dyn ReleaseRepo + Send + Sync>,
    pub config: config::Config,
}

pub fn build_app(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route(
            "/api/me",
            get(handlers::auth::get_me).delete(handlers::auth::delete_me),
        )
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/giphy/search", get(handlers::gif::search_gifs))
        .route(
            "/api/private-messages",
            post(handlers::private_message::send_private_message)
                .get(handlers::private_message::list_private_messages),
        )
        .route(
            "/api/releases",
            post(handlers::release::create_release).get(handlers::release::list_releases),
        )
        .route("/api/releases/{id}", get(handlers::release::get_release))
        .route(
            "/api/releases/{id}/cancel",
            post(handlers::release::cancel_release),
        )
        .route(
            "/api/releases/{id}/steps/{step}/validate",
            post(handlers::release::validate_release_step),
        )
        .route(
            "/api/releases/{id}/incidents/{incident_id}/link",
            post(handlers::release::link_incident).delete(handlers::release::unlink_incident),
        )
        .route(
            "/api/teams",
            post(handlers::team::create_team).get(handlers::team::list_teams),
        )
        .route("/api/teams/join", post(handlers::team::join_team))
        .route(
            "/api/teams/{team_id}/members",
            get(handlers::team::list_members),
        )
        .route(
            "/api/teams/{team_id}/members/{user_id}/role",
            put(handlers::team::set_member_role),
        )
        .route(
            "/api/teams/{team_id}/members/{user_id}",
            delete(handlers::team::kick_member),
        )
        .route(
            "/api/teams/{team_id}/bans",
            post(handlers::team::ban_member).get(handlers::team::list_bans),
        )
        .route(
            "/api/teams/{team_id}/bans/{user_id}",
            delete(handlers::team::unban_member),
        )
        .route(
            "/api/teams/{team_id}/invitation",
            get(handlers::team::get_invitation_code),
        )
        .route("/api/teams/{team_id}", delete(handlers::team::delete_team))
        .route(
            "/api/teams/{team_id}/leave",
            post(handlers::team::leave_team),
        )
        .route(
            "/api/teams/{team_id}/manager",
            put(handlers::team::transfer_manager),
        )
        .route(
            "/api/teams/{team_id}/service-connections",
            get(handlers::team_automation::list_connections),
        )
        .route(
            "/api/teams/{team_id}/service-connections/github",
            put(handlers::team_automation::configure_github),
        )
        .route(
            "/api/teams/{team_id}/service-connections/http",
            put(handlers::team_automation::configure_http),
        )
        .route(
            "/api/teams/{team_id}/service-connections/{connection_id}/test",
            post(handlers::team_automation::test_http_connection),
        )
        .route(
            "/api/teams/{team_id}/service-connections/{connection_id}",
            delete(handlers::team_automation::delete_connection),
        )
        .route(
            "/api/teams/{team_id}/automation-rules",
            get(handlers::team_automation::list_rules).post(handlers::team_automation::create_rule),
        )
        .route(
            "/api/teams/{team_id}/automation-rules/{rule_id}",
            axum::routing::patch(handlers::team_automation::update_rule)
                .delete(handlers::team_automation::delete_rule),
        )
        .route(
            "/api/teams/{team_id}/automation-runs",
            get(handlers::team_automation::list_runs),
        )
        .route(
            "/api/incidents",
            post(handlers::incident::create_incident).get(handlers::incident::list_incidents),
        )
        .route(
            "/api/incidents/{incident_id}",
            delete(handlers::incident::delete_incident).get(handlers::incident::get_incident),
        )
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
        .route(
            "/api/incidents/{incident_id}/activity",
            get(handlers::incident::list_incident_activity),
        )
        .route(
            "/api/incidents/reactions/available",
            get(handlers::incident::available_reactions),
        )
        .route(
            "/api/incidents/{incident_id}/timeline/{entry_id}",
            put(handlers::incident::edit_timeline_entry),
        )
        .route(
            "/api/incidents/{incident_id}/timeline/{entry_id}/reactions",
            post(handlers::incident::toggle_reaction),
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
        .route("/api/auth/google/start", get(handlers::auth::google_start))
        .route(
            "/api/auth/google/callback",
            get(handlers::auth::google_callback),
        )
        // Public upgrade: the WS authenticates in-band via its first message.
        .route("/ws", get(handlers::ws::ws_handler))
        // Public: authenticated by the connection's HMAC signature, not a JWT.
        .route(
            "/webhooks/github/{connection_id}",
            post(handlers::webhook::receive_github_for_connection)
                .layer(DefaultBodyLimit::max(1024 * 1024)),
        )
        .merge(protected_routes)
        .with_state(state)
}
