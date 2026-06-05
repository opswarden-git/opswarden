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
}

pub trait TokenService: Send + Sync {}
pub trait Clock: Send + Sync {}
