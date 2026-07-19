// --- server/src/main.rs ---

use opswarden_server::adapters::crypto::hasher::Argon2Hasher;
use opswarden_server::adapters::crypto::hmac::HmacSha256Verifier;
use opswarden_server::adapters::crypto::jwt::JwtTokenService;
use opswarden_server::adapters::giphy::GiphyClient;
use opswarden_server::adapters::notify::HttpNotifier;
use opswarden_server::adapters::oauth::GoogleOAuthClient;
use opswarden_server::adapters::pg::automation::execution::{
    PgAutomationRunRepo, PgWebhookDeliveryRepo,
};
use opswarden_server::adapters::pg::automation::rule::PgAutomationRuleRepo;
use opswarden_server::adapters::pg::automation::service_connection::{
    PgConnectionCredentialVault, PgServiceConnectionRepo,
};
use opswarden_server::adapters::pg::incident::PgIncidentRepo;
use opswarden_server::adapters::pg::private_message::PgPrivateMessageRepo;
use opswarden_server::adapters::pg::release::PgReleaseRepo;
use opswarden_server::adapters::pg::team::PgTeamRepo;
use opswarden_server::adapters::pg::timeline::PgTimelineRepo;
use opswarden_server::adapters::pg::token_revocation::PgTokenRevocationRepo;
use opswarden_server::adapters::pg::user::PgUserRepo;
use opswarden_server::adapters::webhook::github::GithubParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::ports::Clock;
use opswarden_server::{build_app, config::Config, AppState};

use sqlx::postgres::PgPoolOptions;

use std::sync::Arc;

struct DummyClock;
impl Clock for DummyClock {}

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let state = AppState {
        users: Arc::new(PgUserRepo::new(pool.clone())),
        teams: Arc::new(PgTeamRepo::new(pool.clone())),
        incidents: Arc::new(PgIncidentRepo::new(pool.clone())),
        timeline: Arc::new(PgTimelineRepo::new(pool.clone())),
        private_messages: Arc::new(PgPrivateMessageRepo::new(pool.clone())),
        releases: Arc::new(PgReleaseRepo::new(pool.clone())),
        hasher: Arc::new(Argon2Hasher),
        tokens: Arc::new(JwtTokenService::new(config.jwt_secret.clone())),
        oauth: Arc::new(GoogleOAuthClient::new(
            config.google_oauth_client_id.clone(),
            config.google_oauth_client_secret.clone(),
            config.google_oauth_redirect_uri.clone(),
        )),
        token_revocations: Arc::new(PgTokenRevocationRepo::new(pool.clone())),
        events: Arc::new(WsHub::new()),
        clock: Arc::new(DummyClock),
        webhook_verifier: Arc::new(HmacSha256Verifier),
        webhook_parser: Arc::new(GithubParser),
        service_connections: Arc::new(PgServiceConnectionRepo::new(pool.clone())),
        connection_credentials: Arc::new(PgConnectionCredentialVault::new(
            pool.clone(),
            config.vault_key,
        )),
        automation_rules: Arc::new(PgAutomationRuleRepo::new(pool.clone())),
        webhook_deliveries: Arc::new(PgWebhookDeliveryRepo::new(pool.clone())),
        automation_runs: Arc::new(PgAutomationRunRepo::new(pool.clone())),
        notifier: Arc::new(HttpNotifier::new()),
        gifs: Arc::new(GiphyClient::new(
            config.giphy_api_key.clone(),
            "https://api.giphy.com".to_string(),
        )),
        config,
    };

    let app = build_app(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");
    println!("OpsWarden server listening on {addr}");
    axum::serve(listener, app).await.expect("server error");
}
