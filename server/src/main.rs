// --- server/src/main.rs ---

use opswarden_server::{build_app, config::Config, AppState};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use opswarden_server::ports::{Clock, PasswordHasher, TokenService, UserRepo};

use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::user::User;
use async_trait::async_trait;

struct DummyUserRepo;
#[async_trait]
impl UserRepo for DummyUserRepo {
    async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
        Ok(None)
    }
    async fn save(&self, _user: &User) -> Result<(), DomainError> {
        Ok(())
    }
}

struct DummyHasher;
impl PasswordHasher for DummyHasher {
    fn hash(&self, _password: &str) -> Result<String, DomainError> {
        Ok("dummy_hash".to_string())
    }
}

struct DummyTokenService;
impl TokenService for DummyTokenService {}

struct DummyClock;
impl Clock for DummyClock {}

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string());
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
        users: Arc::new(DummyUserRepo),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        clock: Arc::new(DummyClock),
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
