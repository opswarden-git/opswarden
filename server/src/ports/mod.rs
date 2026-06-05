// --- server/src/ports/mod.rs ---

use async_trait::async_trait;
use crate::domain::user::User;
use crate::domain::error::DomainError;

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
}

pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, DomainError>;
    fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError>;
}

pub trait TokenService: Send + Sync {
    fn generate_token(&self, user_id: uuid::Uuid) -> Result<String, DomainError>;
    fn verify_token(&self, token: &str) -> Result<uuid::Uuid, DomainError>;
}
pub trait Clock: Send + Sync {}
