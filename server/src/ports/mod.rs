// --- server/src/ports/mod.rs ---

use crate::domain::automation::{ExternalEvent, Rule};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::Incident;
use crate::domain::team::{Role, Team};
use crate::domain::timeline::TimelineEntry;
use crate::domain::user::User;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
    /// Every team a user belongs to. Used by the WebSocket hub to register a
    /// connection for the right broadcast scopes at connect time.
    async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
    /// Every team a user belongs to, paired with the role they hold there.
    /// Powers the dashboard's team list and lets the client gate actions by role.
    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>, DomainError>;
    /// Delete a team completely from the system.
    async fn delete_team(&self, team_id: Uuid) -> Result<(), DomainError>;
    /// Remove a user from a team.
    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait IncidentRepo: Send + Sync {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    async fn find_incident_by_id(&self, incident_id: Uuid)
        -> Result<Option<Incident>, DomainError>;
    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError>;
    async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait TimelineRepo: Send + Sync {
    async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError>;
    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError>;
}

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

#[async_trait]
pub trait TokenRevocationRepo: Send + Sync {
    async fn revoke(&self, token: &str, expires_at: DateTime<Utc>) -> Result<(), DomainError>;
    async fn is_revoked(&self, token: &str) -> Result<bool, DomainError>;
}
pub trait Clock: Send + Sync {}

// --- Phase 2: automation & secrets -----------------------------------------

/// Encrypted storage for third-party secrets (webhook HMAC keys, outbound API
/// tokens). The persisted form is ciphertext only — a raw `SELECT` reveals
/// nothing. The cipher (AES-GCM) and the storage backend are adapter concerns.
#[async_trait]
pub trait SecretVault: Send + Sync {
    /// Encrypt and persist the secret for `service` (idempotent upsert).
    async fn store(&self, service: &str, secret: &str) -> Result<(), DomainError>;
    /// Decrypt and return the secret for `service`, or `None` if none is stored.
    async fn reveal(&self, service: &str) -> Result<Option<String>, DomainError>;
}

/// Verifies that an inbound webhook body carries a valid signature for a given
/// shared secret. Implementations are constant-time (HMAC-SHA256 for GitHub).
pub trait WebhookVerifier: Send + Sync {
    fn verify(&self, secret: &str, body: &[u8], signature: &str) -> bool;
}

/// Decodes a raw provider payload into a normalized domain `ExternalEvent`.
/// Returns `None` for payloads we don't act on (so they're acknowledged, not
/// rejected). Provider-specific JSON shapes live in the adapter, never the app.
pub trait WebhookParser: Send + Sync {
    fn parse(&self, service: &str, body: &[u8]) -> Option<ExternalEvent>;
}

/// Supplies the configured automation rules to the hook engine.
#[async_trait]
pub trait RuleRepo: Send + Sync {
    async fn list_rules(&self) -> Result<Vec<Rule>, DomainError>;
}

/// Outbound notification REAction: POST a `message` to a `url`. One generic
/// connector — a Slack incoming webhook, Discord, Teams or any HTTP endpoint is
/// just a URL. The transport (reqwest, payload shape) is an adapter concern.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError>;
}
