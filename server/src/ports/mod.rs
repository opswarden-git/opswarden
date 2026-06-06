// --- server/src/ports/mod.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::{Role, Team};
use crate::domain::user::User;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
}

#[async_trait]
pub trait TeamRepo: Send + Sync {
    /// Persist a newly created team.
    async fn save_team(&self, team: &Team) -> Result<(), DomainError>;
    /// Resolve a team from a (human-typed) invitation code.
    async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError>;
    /// The role a user holds in a team, or `None` if they are not a member.
    /// Lets use-cases enforce RBAC (403) without leaking membership into them.
    async fn find_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, DomainError>;
    /// Add a user to a team with the given role.
    async fn add_member(&self, team_id: Uuid, user_id: Uuid, role: Role)
        -> Result<(), DomainError>;
    /// Atomically demote `old_manager` and promote `new_manager`, upholding the
    /// single-Manager invariant in one transaction.
    async fn transfer_manager(
        &self,
        team_id: Uuid,
        old_manager: Uuid,
        new_manager: Uuid,
    ) -> Result<(), DomainError>;
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
