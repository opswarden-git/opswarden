// --- server/src/main.rs ---

use opentelemetry::KeyValue;
use opentelemetry_sdk::{trace as sdktrace, Resource};
use opswarden_server::adapters::clock::SystemClock;
use opswarden_server::adapters::crypto::hasher::Argon2Hasher;
use opswarden_server::adapters::crypto::hmac::HmacSha256Verifier;
use opswarden_server::adapters::crypto::jwt::JwtTokenService;
use opswarden_server::adapters::email::SmtpEmailSender;
use opswarden_server::adapters::giphy::GiphyClient;
use opswarden_server::adapters::notify::HttpNotifier;
use opswarden_server::adapters::oauth::{GithubServiceOAuthClient, GoogleOAuthClient};
use opswarden_server::adapters::pg::automation::execution::{
    PgAutomationRunRepo, PgWebhookDeliveryRepo,
};
use opswarden_server::adapters::pg::automation::rule::PgAutomationRuleRepo;
use opswarden_server::adapters::pg::automation::service_connection::{
    PgConnectionCredentialVault, PgServiceConnectionRepo,
};
use opswarden_server::adapters::pg::automation::timer::PgAutomationTimerRepo;
use opswarden_server::adapters::pg::channel::PgChannelRepo;
use opswarden_server::adapters::pg::incident::PgIncidentRepo;
use opswarden_server::adapters::pg::private_message::PgPrivateMessageRepo;
use opswarden_server::adapters::pg::release::PgReleaseRepo;
use opswarden_server::adapters::pg::team::PgTeamRepo;
use opswarden_server::adapters::pg::timeline::PgTimelineRepo;
use opswarden_server::adapters::pg::token_revocation::PgTokenRevocationRepo;
use opswarden_server::adapters::pg::user::PgUserRepo;
use opswarden_server::adapters::webhook::CompositeWebhookParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::app::automation::{TimerWorker, TimerWorkerDependencies};
use opswarden_server::ports::Clock;
use opswarden_server::{build_app, config::Config, AppState};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use sqlx::{postgres::PgPoolOptions, PgPool};

use std::{net::SocketAddr, sync::Arc, time::Duration};

const DATABASE_CONNECT_ATTEMPTS: usize = 30;
const DATABASE_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() {
    let migrate_only = std::env::var("OPSWARDEN_MIGRATE_ONLY").as_deref() == Ok("1");
    let skip_migrations = std::env::var("OPSWARDEN_SKIP_MIGRATIONS").as_deref() == Ok("1");
    assert!(
        !(migrate_only && skip_migrations),
        "OPSWARDEN_MIGRATE_ONLY and OPSWARDEN_SKIP_MIGRATIONS are mutually exclusive"
    );

    // Initialize Tracing and OpenTelemetry
    let tracer =
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().tonic())
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", "opswarden-server"),
            ])))
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("failed to install OpenTelemetry tracer");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().json();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(telemetry_layer)
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string());
    let pool = connect_database(&database_url).await;

    if !skip_migrations {
        let mut migrator = sqlx::migrate!();
        migrator.set_ignore_missing(true);
        migrator
            .run(&pool)
            .await
            .expect("Failed to run database migrations");
    }

    if migrate_only {
        pool.close().await;
        return;
    }

    let config = Config::from_env();

    let state = AppState {
        users: Arc::new(PgUserRepo::new(pool.clone())),
        teams: Arc::new(PgTeamRepo::new(pool.clone())),
        channels: Arc::new(PgChannelRepo::new(pool.clone())),
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
        service_oauth: Arc::new(GithubServiceOAuthClient::new(
            config.github_oauth_client_id.clone(),
            config.github_oauth_client_secret.clone(),
            config.github_oauth_redirect_uri.clone(),
        )),
        token_revocations: Arc::new(PgTokenRevocationRepo::new(pool.clone())),
        events: Arc::new(WsHub::new()),
        clock: Arc::new(SystemClock),
        webhook_verifier: Arc::new(HmacSha256Verifier),
        webhook_parser: Arc::new(CompositeWebhookParser::new()),
        service_connections: Arc::new(PgServiceConnectionRepo::new(pool.clone())),
        connection_credentials: Arc::new(PgConnectionCredentialVault::new(
            pool.clone(),
            config.vault_key,
        )),
        automation_rules: Arc::new(PgAutomationRuleRepo::new(pool.clone())),
        webhook_deliveries: Arc::new(PgWebhookDeliveryRepo::new(pool.clone())),
        automation_runs: Arc::new(PgAutomationRunRepo::new(pool.clone())),
        notifier: Arc::new(HttpNotifier::new()),
        email_sender: Arc::new(SmtpEmailSender::new()),
        gifs: Arc::new(GiphyClient::new(
            config.giphy_api_key.clone(),
            "https://api.giphy.com".to_string(),
        )),
        config,
    };

    let timer_poll = Duration::from_secs(state.config.timer_poll_seconds);
    let timer_clock = state.clock.clone();
    let timer_worker = Arc::new(TimerWorker::new(TimerWorkerDependencies {
        timers: Arc::new(PgAutomationTimerRepo::new(pool.clone())),
        connections: state.service_connections.clone(),
        credentials: state.connection_credentials.clone(),
        deliveries: state.webhook_deliveries.clone(),
        rules: state.automation_rules.clone(),
        runs: state.automation_runs.clone(),
        incidents: state.incidents.clone(),
        releases: state.releases.clone(),
        notifier: state.notifier.clone(),
        events: state.events.clone(),
        email_sender: state.email_sender.clone(),
    }));

    let app = build_app(state).layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");
    println!("OpsWarden server listening on {addr}");
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let timer_task = tokio::spawn(run_timer_worker(
        timer_worker,
        timer_clock,
        timer_poll,
        shutdown_rx,
    ));
    let server_result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;
    let _ = shutdown_tx.send(true);
    let _ = timer_task.await;
    server_result.expect("server error");
}

async fn run_timer_worker(
    worker: Arc<TimerWorker>,
    clock: Arc<dyn Clock + Send + Sync>,
    poll_interval: Duration,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(poll_interval);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = clock.now();
                if let Err(error) = worker.reconcile(now).await {
                    tracing::error!(
                        error_code = error.code(),
                        "timer reconciliation failed"
                    );
                }
                match worker.tick(now).await {
                    Ok(result) if result.claimed > 0 => tracing::info!(
                        claimed = result.claimed,
                        succeeded = result.succeeded,
                        failed = result.failed,
                        skipped = result.skipped,
                        "timer tick completed"
                    ),
                    Ok(_) => {}
                    Err(error) => tracing::error!(
                        error_code = error.code(),
                        "timer tick failed"
                    ),
                }
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn connect_database(database_url: &str) -> PgPool {
    for attempt in 1..=DATABASE_CONNECT_ATTEMPTS {
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(_) if attempt < DATABASE_CONNECT_ATTEMPTS => {
                eprintln!(
                    "Postgres connection attempt {attempt}/{DATABASE_CONNECT_ATTEMPTS} failed; \
                     retrying in {}s",
                    DATABASE_CONNECT_RETRY_DELAY.as_secs()
                );
                tokio::time::sleep(DATABASE_CONNECT_RETRY_DELAY).await;
            }
            Err(_) => {
                panic!("Failed to connect to Postgres after {DATABASE_CONNECT_ATTEMPTS} attempts")
            }
        }
    }

    unreachable!("the bounded database connection loop always returns or panics")
}
