// --- server/tests/health.rs ---

use axum::body::Body;
use axum::http::{Request, StatusCode};
use opswarden_server::{build_app, AppState, config::Config};
use opswarden_server::ports::{Clock, PasswordHasher, TokenService, UserRepo};
use std::sync::Arc;
use tower::ServiceExt;

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

fn test_app() -> axum::Router {
    let config = Config::from_env();

    build_app(AppState {
        users: Arc::new(DummyUserRepo),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        clock: Arc::new(DummyClock),
        config,
    })
}

#[tokio::test]
async fn health_returns_ok() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn about_exposes_a_64_char_token() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/about.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token = json["server"]["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
}
