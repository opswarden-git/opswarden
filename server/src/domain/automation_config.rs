// --- server/src/domain/automation_config.rs ---
//
// Durable R9 automation resources. These live beside the legacy in-memory
// `automation::Rule` until the webhook engine is switched to PostgreSQL. The
// types model ownership and lifecycle only; encryption, SQL and provider JSON
// remain adapter concerns.

use std::fmt;

use chrono::{DateTime, Timelike, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::error::DomainError;

pub const MAX_SERVICE_NAME_LEN: usize = 100;
pub const MAX_RULE_NAME_LEN: usize = 200;
pub const MAX_AUTOMATION_KIND_LEN: usize = 100;
pub const MAX_PROVIDER_DELIVERY_ID_LEN: usize = 255;

fn valid_name(value: &str, max_len: usize) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.len() <= max_len
}

fn object_config(value: &Value) -> bool {
    value.is_object() && !contains_sensitive_key(value)
}

fn contains_sensitive_key(value: &Value) -> bool {
    match value {
        Value::Object(values) => values.iter().any(|(key, value)| {
            let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
            normalized.contains("secret")
                || normalized.contains("token")
                || normalized.contains("password")
                || normalized.contains("authorization")
                || normalized == "api_key"
                || normalized == "private_key"
                || normalized == "webhook_url"
                || normalized == "endpoint_url"
                || normalized == "target_url"
                || contains_sensitive_key(value)
        }),
        Value::Array(values) => values.iter().any(contains_sensitive_key),
        _ => false,
    }
}

/// PostgreSQL `timestamptz` stores microseconds. Creating new aggregates at the
/// same precision makes a persist/reload round-trip value-equal instead of
/// silently losing sub-microsecond data at the adapter boundary.
fn now() -> DateTime<Utc> {
    let current = Utc::now();
    current
        .with_nanosecond((current.nanosecond() / 1_000) * 1_000)
        .expect("a truncated nanosecond value is always valid")
}

/// Non-secret metadata for a provider connection owned by exactly one Team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceConnection {
    pub id: Uuid,
    pub team_id: Uuid,
    pub service: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
}

impl ServiceConnection {
    pub fn new(
        team_id: Uuid,
        service: impl Into<String>,
        created_by: Uuid,
    ) -> Result<Self, DomainError> {
        let service = service.into();
        if !valid_name(&service, MAX_SERVICE_NAME_LEN) {
            return Err(DomainError::InvalidServiceConnection);
        }

        let now = now();
        Ok(Self {
            id: Uuid::new_v4(),
            team_id,
            service: service.trim().to_ascii_lowercase(),
            created_by: Some(created_by),
            created_at: now,
            updated_at: now,
            verified_at: None,
            last_delivery_at: None,
            last_error_code: None,
        })
    }
}

/// The purpose of one encrypted value attached to a service connection.
/// Providers may need more than one kind (for example a PAT plus a webhook
/// signing secret), so this is a row discriminator rather than a column name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialKind {
    WebhookSigningSecret,
    PersonalToken,
    OAuthAccessToken,
    OAuthRefreshToken,
    EndpointUrl,
}

impl CredentialKind {
    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        match value {
            "webhook_signing_secret" => Ok(Self::WebhookSigningSecret),
            "personal_token" => Ok(Self::PersonalToken),
            "oauth_access_token" => Ok(Self::OAuthAccessToken),
            "oauth_refresh_token" => Ok(Self::OAuthRefreshToken),
            "endpoint_url" => Ok(Self::EndpointUrl),
            _ => Err(DomainError::Storage),
        }
    }
}

impl fmt::Display for CredentialKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WebhookSigningSecret => "webhook_signing_secret",
            Self::PersonalToken => "personal_token",
            Self::OAuthAccessToken => "oauth_access_token",
            Self::OAuthRefreshToken => "oauth_refresh_token",
            Self::EndpointUrl => "endpoint_url",
        })
    }
}

/// Persisted Action -> REAction rule. Credential values never belong in either
/// JSON config; external targets are referenced through a connection id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationRule {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub trigger_connection_id: Uuid,
    pub trigger_kind: String,
    pub trigger_config: Value,
    pub reaction_kind: String,
    pub reaction_connection_id: Option<Uuid>,
    pub reaction_config: Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationRuleDefinition {
    pub name: String,
    pub trigger_connection_id: Uuid,
    pub trigger_kind: String,
    pub trigger_config: Value,
    pub reaction_kind: String,
    pub reaction_connection_id: Option<Uuid>,
    pub reaction_config: Value,
}

impl AutomationRuleDefinition {
    fn validate(&self) -> Result<(), DomainError> {
        if !valid_name(&self.name, MAX_RULE_NAME_LEN)
            || !valid_name(&self.trigger_kind, MAX_AUTOMATION_KIND_LEN)
            || !valid_name(&self.reaction_kind, MAX_AUTOMATION_KIND_LEN)
            || !object_config(&self.trigger_config)
            || !object_config(&self.reaction_config)
        {
            return Err(DomainError::InvalidAutomationRule);
        }
        Ok(())
    }

    fn normalize(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.trigger_kind = self.trigger_kind.trim().to_string();
        self.reaction_kind = self.reaction_kind.trim().to_string();
        self
    }
}

impl AutomationRule {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        team_id: Uuid,
        name: impl Into<String>,
        trigger_connection_id: Uuid,
        trigger_kind: impl Into<String>,
        trigger_config: Value,
        reaction_kind: impl Into<String>,
        reaction_connection_id: Option<Uuid>,
        reaction_config: Value,
        created_by: Uuid,
    ) -> Result<Self, DomainError> {
        let definition = AutomationRuleDefinition {
            name: name.into(),
            trigger_connection_id,
            trigger_kind: trigger_kind.into(),
            trigger_config,
            reaction_kind: reaction_kind.into(),
            reaction_connection_id,
            reaction_config,
        };
        definition.validate()?;
        let definition = definition.normalize();

        let now = now();
        Ok(Self {
            id: Uuid::new_v4(),
            team_id,
            name: definition.name,
            enabled: false,
            trigger_connection_id: definition.trigger_connection_id,
            trigger_kind: definition.trigger_kind,
            trigger_config: definition.trigger_config,
            reaction_kind: definition.reaction_kind,
            reaction_connection_id: definition.reaction_connection_id,
            reaction_config: definition.reaction_config,
            created_by: Some(created_by),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn definition(&self) -> AutomationRuleDefinition {
        AutomationRuleDefinition {
            name: self.name.clone(),
            trigger_connection_id: self.trigger_connection_id,
            trigger_kind: self.trigger_kind.clone(),
            trigger_config: self.trigger_config.clone(),
            reaction_kind: self.reaction_kind.clone(),
            reaction_connection_id: self.reaction_connection_id,
            reaction_config: self.reaction_config.clone(),
        }
    }

    pub fn replace_definition(
        &mut self,
        definition: AutomationRuleDefinition,
    ) -> Result<(), DomainError> {
        definition.validate()?;
        let definition = definition.normalize();
        self.name = definition.name;
        self.trigger_connection_id = definition.trigger_connection_id;
        self.trigger_kind = definition.trigger_kind;
        self.trigger_config = definition.trigger_config;
        self.reaction_kind = definition.reaction_kind;
        self.reaction_connection_id = definition.reaction_connection_id;
        self.reaction_config = definition.reaction_config;
        self.updated_at = now();
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.updated_at = now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookDeliveryStatus {
    Received,
    Ignored,
    Processed,
    Failed,
}

impl WebhookDeliveryStatus {
    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        match value {
            "received" => Ok(Self::Received),
            "ignored" => Ok(Self::Ignored),
            "processed" => Ok(Self::Processed),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Storage),
        }
    }

    pub fn is_terminal(self) -> bool {
        self != Self::Received
    }
}

impl fmt::Display for WebhookDeliveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Received => "received",
            Self::Ignored => "ignored",
            Self::Processed => "processed",
            Self::Failed => "failed",
        })
    }
}

/// Compact provider delivery ledger. The raw request body and headers are not
/// retained, preventing secrets and personal data from becoming an audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub provider_delivery_id: String,
    pub provider_event: String,
    pub status: WebhookDeliveryStatus,
    pub error_code: Option<String>,
    pub received_at: DateTime<Utc>,
}

impl WebhookDelivery {
    pub fn new(
        connection_id: Uuid,
        provider_delivery_id: impl Into<String>,
        provider_event: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let provider_delivery_id = provider_delivery_id.into();
        let provider_event = provider_event.into();
        if !valid_name(&provider_delivery_id, MAX_PROVIDER_DELIVERY_ID_LEN)
            || !valid_name(&provider_event, MAX_AUTOMATION_KIND_LEN)
        {
            return Err(DomainError::InvalidWebhookDelivery);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            connection_id,
            provider_delivery_id: provider_delivery_id.trim().to_string(),
            provider_event: provider_event.trim().to_string(),
            status: WebhookDeliveryStatus::Received,
            error_code: None,
            received_at: now(),
        })
    }

    pub fn mark_processed(&mut self) -> Result<(), DomainError> {
        self.finish(WebhookDeliveryStatus::Processed, None)
    }

    pub fn mark_ignored(&mut self) -> Result<(), DomainError> {
        self.finish(WebhookDeliveryStatus::Ignored, None)
    }

    pub fn mark_failed(&mut self, error_code: impl Into<String>) -> Result<(), DomainError> {
        let error_code = error_code.into();
        if !valid_name(&error_code, MAX_PROVIDER_DELIVERY_ID_LEN) {
            return Err(DomainError::InvalidWebhookDelivery);
        }
        self.finish(
            WebhookDeliveryStatus::Failed,
            Some(error_code.trim().to_string()),
        )
    }

    fn finish(
        &mut self,
        status: WebhookDeliveryStatus,
        error_code: Option<String>,
    ) -> Result<(), DomainError> {
        if self.status.is_terminal() || status == WebhookDeliveryStatus::Received {
            return Err(DomainError::InvalidAutomationTransition);
        }
        self.status = status;
        self.error_code = error_code;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationRunStatus {
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl AutomationRunStatus {
    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        match value {
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(DomainError::Storage),
        }
    }
}

impl fmt::Display for AutomationRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        })
    }
}

/// Durable result of evaluating one rule for one provider delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationRun {
    pub id: Uuid,
    pub delivery_id: Uuid,
    /// Becomes `None` if the historical rule is later deleted.
    pub rule_id: Option<Uuid>,
    pub status: AutomationRunStatus,
    pub incident_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl AutomationRun {
    pub fn new(delivery_id: Uuid, rule_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            delivery_id,
            rule_id: Some(rule_id),
            status: AutomationRunStatus::Running,
            incident_id: None,
            error_code: None,
            started_at: now(),
            finished_at: None,
        }
    }

    pub fn mark_succeeded(&mut self, incident_id: Option<Uuid>) -> Result<(), DomainError> {
        self.finish(AutomationRunStatus::Succeeded, incident_id, None)
    }

    pub fn mark_failed(&mut self, error_code: impl Into<String>) -> Result<(), DomainError> {
        let error_code = error_code.into();
        if !valid_name(&error_code, MAX_PROVIDER_DELIVERY_ID_LEN) {
            return Err(DomainError::InvalidAutomationRun);
        }
        self.finish(
            AutomationRunStatus::Failed,
            None,
            Some(error_code.trim().to_string()),
        )
    }

    pub fn mark_skipped(&mut self) -> Result<(), DomainError> {
        self.finish(AutomationRunStatus::Skipped, None, None)
    }

    fn finish(
        &mut self,
        status: AutomationRunStatus,
        incident_id: Option<Uuid>,
        error_code: Option<String>,
    ) -> Result<(), DomainError> {
        if self.status != AutomationRunStatus::Running {
            return Err(DomainError::InvalidAutomationTransition);
        }
        self.status = status;
        self.incident_id = incident_id;
        self.error_code = error_code;
        self.finished_at = Some(now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connection_normalizes_provider_name() {
        let connection =
            ServiceConnection::new(Uuid::new_v4(), " GitHub ", Uuid::new_v4()).unwrap();
        assert_eq!(connection.service, "github");
    }

    #[test]
    fn new_rule_is_disabled_and_requires_object_configs() {
        let team_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let connection_id = Uuid::new_v4();
        let rule = AutomationRule::new(
            team_id,
            "CI failed",
            connection_id,
            "ci_failed",
            json!({"repository": "opswarden/app"}),
            "vigil_create_incident",
            None,
            json!({"severity": "high"}),
            user_id,
        )
        .unwrap();
        assert!(!rule.enabled);

        assert_eq!(
            AutomationRule::new(
                team_id,
                "bad",
                connection_id,
                "ci_failed",
                json!([]),
                "vigil_create_incident",
                None,
                json!({}),
                user_id,
            )
            .unwrap_err(),
            DomainError::InvalidAutomationRule
        );

        assert_eq!(
            AutomationRule::new(
                team_id,
                "leaky",
                connection_id,
                "ci_failed",
                json!({"nested": {"access_token": "must-not-live-here"}}),
                "vigil_create_incident",
                None,
                json!({}),
                user_id,
            )
            .unwrap_err(),
            DomainError::InvalidAutomationRule
        );
    }

    #[test]
    fn delivery_and_run_are_single_transition_state_machines() {
        let mut delivery =
            WebhookDelivery::new(Uuid::new_v4(), "delivery-1", "workflow_run").unwrap();
        delivery.mark_processed().unwrap();
        assert_eq!(
            delivery.mark_failed("late_failure").unwrap_err(),
            DomainError::InvalidAutomationTransition
        );

        let mut run = AutomationRun::new(delivery.id, Uuid::new_v4());
        let incident_id = Uuid::new_v4();
        run.mark_succeeded(Some(incident_id)).unwrap();
        assert_eq!(run.incident_id, Some(incident_id));
        assert!(run.finished_at.is_some());
        assert_eq!(
            run.mark_skipped().unwrap_err(),
            DomainError::InvalidAutomationTransition
        );
    }
}
