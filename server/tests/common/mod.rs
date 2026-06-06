// server/tests/common/mod.rs

use async_trait::async_trait;
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::user::User;
use opswarden_server::ports::{Clock, PasswordHasher, TokenService, UserRepo};
use opswarden_server::{build_app, config::Config, AppState};
use std::sync::Arc;

pub struct DummyUserRepo;
#[async_trait]
impl UserRepo for DummyUserRepo {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        if email == "existing@test.com" {
            let e = opswarden_server::domain::user::Email::new(email.to_string()).unwrap();
            Ok(Some(User::new(e, "hash")))
        } else {
            Ok(None)
        }
    }
    async fn save(&self, _user: &User) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct DummyHasher;
impl PasswordHasher for DummyHasher {
    fn hash(&self, _password: &str) -> Result<String, DomainError> {
        Ok("dummy_hash".to_string())
    }
    fn verify(&self, password: &str, _hash: &str) -> Result<bool, DomainError> {
        Ok(password == "correct_password")
    }
}

pub struct DummyTokenService;
impl TokenService for DummyTokenService {
    fn generate_token(&self, _user_id: uuid::Uuid) -> Result<String, DomainError> {
        Ok("mock_jwt_token".to_string())
    }
    fn verify_token(&self, token: &str) -> Result<uuid::Uuid, DomainError> {
        if token == "mock_jwt_token" {
            Ok(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
        } else {
            Err(DomainError::InvalidToken)
        }
    }
}

pub struct DummyClock;
impl Clock for DummyClock {}

pub fn test_app() -> axum::Router {
    let config = Config::from_env();

    build_app(AppState {
        users: Arc::new(DummyUserRepo),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        clock: Arc::new(DummyClock),
        config,
    })
}
