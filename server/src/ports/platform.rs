use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;

#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Broadcast a domain event to interested clients. Fire-and-forget: a
    /// delivery failure must never fail or roll back the business operation that
    /// produced the event.
    async fn publish(&self, event: DomainEvent);
}

pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, DomainError>;
    fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenClaims {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

pub trait TokenService: Send + Sync {
    fn generate_token(&self, user_id: uuid::Uuid) -> Result<String, DomainError>;
    fn verify_token(&self, token: &str) -> Result<TokenClaims, DomainError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthProfile {
    pub email: String,
}

#[async_trait]
pub trait OAuthClient: Send + Sync {
    fn is_configured(&self) -> bool;
    fn authorization_url(&self, state: &str) -> Result<String, DomainError>;
    async fn exchange_code(&self, code: &str) -> Result<OAuthProfile, DomainError>;
}

/// OAuth credentials returned by an external automation provider. Deliberately
/// does not implement `Debug` or serialization: token material may only travel
/// from the provider adapter to the encrypted credential vault.
#[derive(Clone, PartialEq, Eq)]
pub struct ServiceOAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[async_trait]
pub trait ServiceOAuthClient: Send + Sync {
    fn is_configured(&self) -> bool;
    fn authorization_url(&self, state: &str, code_challenge: &str) -> Result<String, DomainError>;
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<ServiceOAuthTokens, DomainError>;
    async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<ServiceOAuthTokens, DomainError>;
}

#[async_trait]
pub trait TokenRevocationRepo: Send + Sync {
    async fn revoke(&self, token: &str, expires_at: DateTime<Utc>) -> Result<(), DomainError>;
    async fn is_revoked(&self, token: &str) -> Result<bool, DomainError>;
}
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

// --- Phase 2: automation & secrets -----------------------------------------
