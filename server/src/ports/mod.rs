// --- server/src/ports/mod.rs ---

use crate::domain::automation::{ExternalEvent, Rule};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::Incident;
use crate::domain::team::{Role, Team, TeamBan, TeamMemberView};
use crate::domain::timeline::{ReactionRecord, TimelineEntry};
use crate::domain::user::User;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError>;
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
    /// Count how many members a team has.
    async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError>;
    /// Every member of a team, enriched with the user's email and role. Powers
    /// the team roster view; the read is scoped to one team by the caller.
    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError>;
    /// Set a member's role within a team. Used for Observer↔Responder changes;
    /// the Manager seat is upheld by `transfer_manager`, not this method.
    async fn set_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError>;
    /// Record (or replace) a moderation ban. Upserts on `(team_id, user_id)` so
    /// re-banning a user updates the existing row rather than duplicating it.
    async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError>;
    /// The ban currently recorded for a user on a team, if any. The row may be
    /// expired; the caller decides via `TeamBan::is_active`.
    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError>;
    /// Every ban recorded for a team, for the Manager's moderation list.
    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBan>, DomainError>;
}

#[async_trait]
pub trait IncidentRepo: Send + Sync {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    async fn find_incident_by_id(&self, incident_id: Uuid)
        -> Result<Option<Incident>, DomainError>;
    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError>;
    async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError>;
    /// Clear the assignee on every incident of `team_id` currently assigned to
    /// `user_id`. Called when a member is kicked/banned so no incident stays
    /// assigned to a non-member (upholds the assignee-must-be-a-member rule).
    async fn clear_assignee_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait TimelineRepo: Send + Sync {
    async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError>;
    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError>;
    /// Load a single entry (to authorize and apply an edit).
    async fn find_entry_by_id(&self, entry_id: Uuid) -> Result<Option<TimelineEntry>, DomainError>;
    /// Persist an edited entry: updates `content` and `edited_at`.
    async fn update_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError>;
    /// Add a reaction; returns `true` when newly inserted, `false` when the user
    /// already had that emoji on the entry (idempotent — no duplicate).
    async fn add_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, DomainError>;
    /// Remove a reaction (idempotent: removing a missing one is not an error).
    async fn remove_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<(), DomainError>;
    /// How many distinct users currently react to `entry_id` with `emoji`.
    async fn count_reaction(&self, entry_id: Uuid, emoji: &str) -> Result<u64, DomainError>;
    /// Every reaction on every entry of an incident, for roster aggregation.
    async fn list_reactions_for_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<ReactionRecord>, DomainError>;
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
    /// Remove the stored secret for `service` (idempotent: deleting a missing
    /// service is not an error).
    async fn delete(&self, service: &str) -> Result<(), DomainError>;
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

/// A normalized GIF search result, independent of the provider's JSON shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GifResult {
    pub id: String,
    pub title: String,
    /// URL of the animated GIF to display when selected.
    pub url: String,
    /// Smaller still/preview URL for the results grid.
    pub preview_url: String,
    pub width: u32,
    pub height: u32,
}

/// External GIF search (RTC2 `web_api_integration`, backed by GIPHY). The
/// provider, HTTP transport and API-key handling are adapter concerns; the
/// use-case only ever sees normalized `GifResult`s.
#[async_trait]
pub trait GifSearch: Send + Sync {
    async fn search(
        &self,
        query: &str,
        limit: u32,
        rating: &str,
    ) -> Result<Vec<GifResult>, DomainError>;
}
