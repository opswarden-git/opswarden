use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{
    AutomationRule, AutomationRun, CredentialKind, ServiceConnection, WebhookDelivery,
};
use crate::domain::automation_timer::{ClaimedTimerOccurrence, TimerSchedule};
use crate::domain::error::DomainError;

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
    async fn update_rule(
        &self,
        rule: &AutomationRule,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;
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
    async fn next_run_at(
        &self,
        _team_id: Uuid,
        _rule_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        Ok(None)
    }
}

/// Durable Timer projection and cross-replica occurrence claims.
#[async_trait]
pub trait AutomationTimerRepo: Send + Sync {
    /// Insert or replace the projection only when the referenced rule is an
    /// enabled Timer rule with the same source revision.
    async fn upsert_schedule(
        &self,
        rule_id: Uuid,
        schedule: &TimerSchedule,
        next_run_at: DateTime<Utc>,
        rule_updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    async fn delete_schedule(&self, rule_id: Uuid) -> Result<bool, DomainError>;

    /// Claim at most one due occurrence. Implementations must persist the
    /// delivery, occurrence key and next-run advancement atomically.
    async fn claim_due(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<ClaimedTimerOccurrence>, DomainError>;

    /// Recheck the rule revision and atomically turn a claim into a running
    /// automation run. Returns false when a disable/edit won the race.
    async fn start_execution(
        &self,
        claim: &ClaimedTimerOccurrence,
        run: &AutomationRun,
    ) -> Result<bool, DomainError>;

    async fn list_unstarted_claims(
        &self,
        claimed_before: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<ClaimedTimerOccurrence>, DomainError>;

    async fn abandon_claim(
        &self,
        claim: &ClaimedTimerOccurrence,
        finished_at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    async fn finalize_stale_runs(
        &self,
        started_before: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Result<u64, DomainError>;
}

/// Idempotency ledger for inbound provider deliveries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebhookDeliveryClaim {
    pub delivery_id: Uuid,
    pub token: Uuid,
}

#[async_trait]
pub trait WebhookDeliveryRepo: Send + Sync {
    /// Atomically claims a new delivery or reclaims an expired attempt.
    /// Returns `None` while another attempt owns the lease or after the
    /// delivery reached a terminal state.
    async fn claim_delivery(
        &self,
        delivery: &WebhookDelivery,
    ) -> Result<Option<WebhookDeliveryClaim>, DomainError>;
    async fn complete_claimed_delivery(
        &self,
        delivery: &WebhookDelivery,
        claim: WebhookDeliveryClaim,
    ) -> Result<bool, DomainError>;
    async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<bool, DomainError>;
    async fn list_deliveries_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebhookJob {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub expected_service: String,
    pub provider_delivery_id: String,
    pub provider_event: String,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimedWebhookJob {
    pub job: WebhookJob,
    pub token: Uuid,
}

#[async_trait]
pub trait WebhookJobRepo: Send + Sync {
    async fn enqueue(&self, job: &WebhookJob) -> Result<bool, DomainError>;
    async fn enqueue_batch(&self, jobs: &[WebhookJob]) -> Result<Vec<bool>, DomainError>;
    async fn claim(&self, limit: u32) -> Result<Vec<ClaimedWebhookJob>, DomainError>;
    async fn complete(&self, claim: &ClaimedWebhookJob) -> Result<bool, DomainError>;
    async fn retry(&self, claim: &ClaimedWebhookJob, error_code: &str)
        -> Result<bool, DomainError>;
}

/// Durable result of running one rule for one delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomationRunReservation {
    New(AutomationRun),
    Existing(AutomationRun),
}

#[async_trait]
pub trait AutomationRunRepo: Send + Sync {
    /// Insert a new run or return the run already reserved for this
    /// `(delivery, rule)` pair after a worker restart.
    async fn reserve_run(
        &self,
        run: &AutomationRun,
        claim: WebhookDeliveryClaim,
    ) -> Result<AutomationRunReservation, DomainError>;
    /// Terminalize runs left `running` by an expired delivery attempt, even if
    /// their rule has since been disabled or deleted.
    async fn interrupt_running_for_delivery(
        &self,
        claim: WebhookDeliveryClaim,
    ) -> Result<u64, DomainError>;
    async fn update_run(&self, run: &AutomationRun) -> Result<bool, DomainError>;
    async fn list_runs_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<AutomationRun>, DomainError>;
}

/// Verifies provider webhook credentials in constant time.
pub trait WebhookVerifier: Send + Sync {
    fn verify(&self, secret: &str, body: &[u8], signature: &str) -> bool;
    fn verify_token(&self, secret: &str, token: &str) -> bool;
}

/// Decodes a raw provider payload into a normalized domain `ExternalEvent`.
/// Returns `None` for payloads we don't act on (so they're acknowledged, not
/// rejected). Provider-specific JSON shapes live in the adapter, never the app.
pub trait WebhookParser: Send + Sync {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent>;
}
