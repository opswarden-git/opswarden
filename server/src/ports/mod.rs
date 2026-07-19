// --- server/src/ports/mod.rs ---

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{
    AutomationRule, AutomationRun, CredentialKind, ServiceConnection, WebhookDelivery,
};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::Incident;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::private_message::PrivateMessage;
use crate::domain::release::{Release, ReleaseState};
use crate::domain::team::{Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamMemberView};
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
    /// Directory read model with operational counters for every team the user
    /// belongs to.
    async fn list_team_directory_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TeamDirectoryItem>, DomainError>;
    /// Resolve a team by its technical id for scoped detail endpoints.
    async fn find_team_by_id(&self, team_id: Uuid) -> Result<Option<Team>, DomainError>;
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
    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError>;
    /// Explicitly lift a ban. Expired rows may also be removed to keep the
    /// moderation history intentional rather than silently reactivatable.
    async fn remove_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait IncidentRepo: Send + Sync {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    /// Persist the initial incident and its audit event atomically.
    async fn save_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError>;
    async fn find_incident_by_id(&self, incident_id: Uuid)
        -> Result<Option<Incident>, DomainError>;
    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    /// Persist a mutation and the event describing it in one transaction.
    async fn update_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError>;
    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError>;
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
pub trait PrivateMessageRepo: Send + Sync {
    /// Persist a sent private message.
    async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError>;
    /// The conversation between two users — both directions of the pair — newest
    /// first, capped at `limit`. The pair is symmetric, so the argument order
    /// does not matter.
    async fn list_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError>;
}

#[async_trait]
pub trait ReleaseRepo: Send + Sync {
    /// Persist a new release and all its (unvalidated) steps.
    async fn save_release(&self, release: &Release) -> Result<(), DomainError>;
    /// Load a release with its ordered steps, or `None`.
    async fn find_release_by_id(&self, release_id: Uuid) -> Result<Option<Release>, DomainError>;
    /// Every release of a team (with steps), newest first.
    async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError>;
    /// Persist a mutated release: its `base_state` and the validation of its steps.
    async fn update_release(&self, release: &Release) -> Result<(), DomainError>;
    /// Link an incident to a release (idempotent on the pair).
    async fn link_incident(&self, release_id: Uuid, incident_id: Uuid) -> Result<(), DomainError>;
    /// Unlink an incident from a release (idempotent: unlinking a missing pair is
    /// not an error).
    async fn unlink_incident(&self, release_id: Uuid, incident_id: Uuid)
        -> Result<(), DomainError>;
    /// The incidents currently linked to a release, for the read view.
    async fn list_linked_incident_ids(&self, release_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
    /// How many of a release's linked incidents are still active (not resolved).
    /// `> 0` is exactly the "is it blocked?" input for `effective_release_state`.
    async fn count_active_linked_incidents(&self, release_id: Uuid) -> Result<u64, DomainError>;
    /// `(release_id, team_id, base_state)` of every release linked to an incident.
    /// Lets an incident status change recompute the blocking of affected releases
    /// without loading their full aggregates.
    async fn list_release_states_linked_to_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, ReleaseState)>, DomainError>;
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

/// Non-secret metadata for provider connections owned by a Team. Every lookup
/// used by authenticated application code carries `team_id` explicitly so an
/// unscoped list cannot be called by accident.
#[async_trait]
pub trait ServiceConnectionRepo: Send + Sync {
    async fn insert_connection(&self, connection: &ServiceConnection) -> Result<(), DomainError>;
    /// Public webhook routing starts from an opaque connection UUID, before a
    /// Team id is known. Authenticated API reads keep using the scoped methods.
    async fn find_connection_by_id(
        &self,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError>;
    async fn find_connection_for_team(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError>;
    async fn find_connection_by_service(
        &self,
        team_id: Uuid,
        service: &str,
    ) -> Result<Option<ServiceConnection>, DomainError>;
    async fn list_connections_for_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<ServiceConnection>, DomainError>;
    /// Record health only after a request passed provider authentication.
    async fn record_delivery_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError>;
    /// Record an outbound check or reaction without changing inbound delivery
    /// timestamps. A successful result verifies the destination once.
    async fn record_reaction_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError>;
    /// A replaced credential must not inherit the verification state of the
    /// previous remote endpoint.
    async fn reset_connection_health(&self, connection_id: Uuid) -> Result<(), DomainError>;
    async fn delete_connection(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<bool, DomainError>;
}

/// Encrypted values attached to a connection. This is intentionally separate
/// from `ServiceConnectionRepo`: ordinary metadata reads have no API capable of
/// returning credential material.
#[async_trait]
pub trait ConnectionCredentialVault: Send + Sync {
    async fn store_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
        secret: &str,
    ) -> Result<(), DomainError>;
    async fn reveal_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<Option<String>, DomainError>;
    async fn delete_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<(), DomainError>;
    async fn configured_credential_kinds(
        &self,
        connection_id: Uuid,
    ) -> Result<Vec<CredentialKind>, DomainError>;
}

/// Durable, Team-owned Action -> REAction rules.
#[async_trait]
pub trait AutomationRuleRepo: Send + Sync {
    async fn insert_rule(&self, rule: &AutomationRule) -> Result<(), DomainError>;
    async fn update_rule(&self, rule: &AutomationRule) -> Result<bool, DomainError>;
    async fn find_rule_for_team(
        &self,
        team_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Option<AutomationRule>, DomainError>;
    async fn list_rules_for_team(&self, team_id: Uuid) -> Result<Vec<AutomationRule>, DomainError>;
    async fn list_enabled_rules_for_trigger(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
        trigger_kind: &str,
    ) -> Result<Vec<AutomationRule>, DomainError>;
    async fn delete_rule(&self, team_id: Uuid, rule_id: Uuid) -> Result<bool, DomainError>;
}

/// Idempotency ledger for inbound provider deliveries.
#[async_trait]
pub trait WebhookDeliveryRepo: Send + Sync {
    /// Atomically reserve a provider delivery. Returns false when this
    /// connection already received the same provider id.
    async fn reserve_delivery(&self, delivery: &WebhookDelivery) -> Result<bool, DomainError>;
    async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<bool, DomainError>;
    async fn list_deliveries_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;
}

/// Durable result of running one rule for one delivery.
#[async_trait]
pub trait AutomationRunRepo: Send + Sync {
    async fn insert_run(&self, run: &AutomationRun) -> Result<(), DomainError>;
    async fn update_run(&self, run: &AutomationRun) -> Result<bool, DomainError>;
    async fn list_runs_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<AutomationRun>, DomainError>;
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
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent>;
}

/// Outbound notification REAction: POST a `message` to a `url`. One generic
/// connector — a Slack incoming webhook, Discord, Teams or any HTTP endpoint is
/// just a URL. The transport (reqwest, payload shape) is an adapter concern.
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Validate syntax, DNS and the resolved network target without sending a
    /// business notification. Configuration and execution share this boundary.
    async fn validate_endpoint(&self, url: &str) -> Result<(), DomainError>;
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
