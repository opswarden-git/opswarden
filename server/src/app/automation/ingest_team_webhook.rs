use std::sync::Arc;

use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{
    AutomationRule, AutomationRun, CredentialKind, WebhookDelivery,
};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::ports::{
    AutomationRuleRepo, AutomationRunRepo, ConnectionCredentialVault, EventPublisher, IncidentRepo,
    Notifier, ServiceConnectionRepo, WebhookDeliveryRepo, WebhookParser, WebhookVerifier,
};

use super::reaction_executor::AutomationReactionExecutor;

pub struct IngestTeamWebhookCommand {
    pub connection_id: Uuid,
    pub provider_delivery_id: String,
    pub provider_event: String,
    pub signature: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IngestTeamWebhookResult {
    pub duplicate: bool,
    pub rules_triggered: usize,
    pub rules_failed: usize,
}

pub struct TeamWebhookDependencies {
    pub connections: Arc<dyn ServiceConnectionRepo>,
    pub credentials: Arc<dyn ConnectionCredentialVault>,
    pub verifier: Arc<dyn WebhookVerifier>,
    pub parser: Arc<dyn WebhookParser>,
    pub deliveries: Arc<dyn WebhookDeliveryRepo>,
    pub rules: Arc<dyn AutomationRuleRepo>,
    pub runs: Arc<dyn AutomationRunRepo>,
    pub incidents: Arc<dyn IncidentRepo>,
    pub notifier: Arc<dyn Notifier>,
    pub events: Arc<dyn EventPublisher>,
}

pub struct IngestTeamWebhookUseCase {
    dependencies: TeamWebhookDependencies,
}

impl IngestTeamWebhookUseCase {
    pub fn new(dependencies: TeamWebhookDependencies) -> Self {
        Self { dependencies }
    }

    pub async fn ingest(
        &self,
        cmd: IngestTeamWebhookCommand,
    ) -> Result<IngestTeamWebhookResult, DomainError> {
        let connection = self
            .dependencies
            .connections
            .find_connection_by_id(cmd.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if connection.service != "github" {
            return Err(DomainError::UnknownService);
        }

        let secret = self
            .dependencies
            .credentials
            .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
            .await?
            .ok_or(DomainError::UnknownService)?;
        if !self.dependencies.verifier.verify(
            &secret,
            &cmd.body,
            cmd.signature.as_deref().unwrap_or_default(),
        ) {
            return Err(DomainError::InvalidSignature);
        }
        let mut delivery =
            WebhookDelivery::new(connection.id, cmd.provider_delivery_id, cmd.provider_event)?;
        if !self
            .dependencies
            .deliveries
            .reserve_delivery(&delivery)
            .await?
        {
            return Ok(IngestTeamWebhookResult {
                duplicate: true,
                rules_triggered: 0,
                rules_failed: 0,
            });
        }
        // A new, correctly signed provider delivery proves the connection.
        // Duplicate retries deliberately keep the original delivery health.
        self.dependencies
            .connections
            .record_delivery_result(connection.id, None)
            .await?;

        let Some(event) = self.dependencies.parser.parse(
            &connection.service,
            &delivery.provider_event,
            &cmd.body,
        ) else {
            delivery.mark_ignored()?;
            self.persist_delivery(&delivery).await?;
            return Ok(IngestTeamWebhookResult {
                duplicate: false,
                rules_triggered: 0,
                rules_failed: 0,
            });
        };

        let rules = self
            .dependencies
            .rules
            .list_enabled_rules_for_trigger(connection.team_id, connection.id, &event.kind)
            .await?;
        let matching_rules: Vec<_> = rules
            .into_iter()
            .filter(|rule| trigger_matches(rule, &event))
            .collect();

        let mut rules_triggered = 0;
        let mut rules_failed = 0;
        let mut first_error_code = None;
        let executor = AutomationReactionExecutor::new(
            self.dependencies.connections.clone(),
            self.dependencies.credentials.clone(),
            self.dependencies.incidents.clone(),
            self.dependencies.notifier.clone(),
        );
        for rule in matching_rules {
            let mut run = AutomationRun::new(delivery.id, rule.id);
            self.dependencies.runs.insert_run(&run).await?;
            match executor.execute(connection.team_id, &rule, &event).await {
                Ok(incident_id) => {
                    run.mark_succeeded(incident_id)?;
                    self.persist_run(&run).await?;
                    rules_triggered += 1;
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleTriggered {
                            team_id: connection.team_id,
                            service: connection.service.clone(),
                            rule: rule.name,
                            incident_id,
                        })
                        .await;
                }
                Err(error) => {
                    let error_code = error.code();
                    run.mark_failed(error_code)?;
                    self.persist_run(&run).await?;
                    rules_failed += 1;
                    first_error_code.get_or_insert(error_code);
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleFailed {
                            team_id: connection.team_id,
                            service: connection.service.clone(),
                            rule: rule.name,
                            reason: error.to_string(),
                        })
                        .await;
                }
            }
        }

        if rules_triggered == 0 && rules_failed == 0 {
            delivery.mark_ignored()?;
        } else {
            delivery.mark_processed()?;
        }
        self.persist_delivery(&delivery).await?;
        if first_error_code.is_some() {
            self.dependencies
                .connections
                .record_delivery_result(connection.id, first_error_code)
                .await?;
        }

        Ok(IngestTeamWebhookResult {
            duplicate: false,
            rules_triggered,
            rules_failed,
        })
    }

    async fn persist_delivery(&self, delivery: &WebhookDelivery) -> Result<(), DomainError> {
        if !self
            .dependencies
            .deliveries
            .update_delivery(delivery)
            .await?
        {
            return Err(DomainError::InvalidAutomationTransition);
        }
        Ok(())
    }

    async fn persist_run(&self, run: &AutomationRun) -> Result<(), DomainError> {
        if !self.dependencies.runs.update_run(run).await? {
            return Err(DomainError::InvalidAutomationTransition);
        }
        Ok(())
    }
}

fn trigger_matches(rule: &AutomationRule, event: &ExternalEvent) -> bool {
    let Some(filters) = rule.trigger_config.as_object() else {
        return false;
    };
    filters
        .iter()
        .all(|(key, expected)| event.attributes.get(key) == Some(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};

    #[test]
    fn trigger_filters_are_an_exact_subset_of_normalized_attributes() {
        let team_id = Uuid::new_v4();
        let connection_id = Uuid::new_v4();
        let rule = AutomationRule::new(
            team_id,
            "Production main",
            connection_id,
            "ci_failed",
            json!({"repository": "opswarden/app", "branch": "main"}),
            "vigil_create_incident",
            None,
            json!({}),
            Uuid::new_v4(),
        )
        .unwrap();
        let attributes: Map<String, Value> = serde_json::from_value(json!({
            "repository": "opswarden/app",
            "branch": "main",
            "workflow": "CI"
        }))
        .unwrap();
        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
        assert!(trigger_matches(&rule, &event));

        let mut other = event.clone();
        other.attributes.insert("branch".into(), json!("develop"));
        assert!(!trigger_matches(&rule, &other));
    }
}
