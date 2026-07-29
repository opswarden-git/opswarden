use std::sync::Arc;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{AutomationRun, ServiceConnection, WebhookDelivery};
use crate::domain::error::DomainError;
use crate::domain::event::{AutomationRuleResult, DomainEvent};
use crate::domain::release::Release;
use crate::ports::{
    AutomationRuleRepo, AutomationRunRepo, ConnectionCredentialVault, EventPublisher, IncidentRepo,
    Notifier, ReleaseRepo, ServiceConnectionRepo, WebhookDeliveryRepo,
};

use super::ingest_team_webhook::trigger_matches;
use super::AutomationReactionExecutor;

pub const OPSWARDEN_SERVICE: &str = "opswarden";

pub struct DispatchInternalAutomationCommand {
    pub team_id: Uuid,
    pub delivery_id: String,
    pub event: ExternalEvent,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DispatchInternalAutomationResult {
    pub duplicate: bool,
    pub rules_triggered: usize,
    pub rules_failed: usize,
}

pub struct InternalAutomationDependencies {
    pub connections: Arc<dyn ServiceConnectionRepo>,
    pub credentials: Arc<dyn ConnectionCredentialVault>,
    pub deliveries: Arc<dyn WebhookDeliveryRepo>,
    pub rules: Arc<dyn AutomationRuleRepo>,
    pub runs: Arc<dyn AutomationRunRepo>,
    pub incidents: Arc<dyn IncidentRepo>,
    pub releases: Arc<dyn ReleaseRepo>,
    pub notifier: Arc<dyn Notifier>,
    pub events: Arc<dyn EventPublisher>,
}

pub struct DispatchInternalAutomationUseCase {
    dependencies: InternalAutomationDependencies,
}

impl DispatchInternalAutomationUseCase {
    pub fn new(dependencies: InternalAutomationDependencies) -> Self {
        Self { dependencies }
    }

    pub async fn dispatch(
        &self,
        cmd: DispatchInternalAutomationCommand,
    ) -> Result<DispatchInternalAutomationResult, DomainError> {
        if cmd.event.service != OPSWARDEN_SERVICE {
            return Err(DomainError::InvalidAutomationRule);
        }
        let connection = match self
            .dependencies
            .connections
            .find_connection_by_service(cmd.team_id, OPSWARDEN_SERVICE)
            .await?
        {
            Some(connection) => connection,
            None => {
                let connection = ServiceConnection::new_internal(cmd.team_id, OPSWARDEN_SERVICE)?;
                self.dependencies
                    .connections
                    .insert_connection(&connection)
                    .await?;
                connection
            }
        };
        let mut delivery = WebhookDelivery::new(connection.id, cmd.delivery_id, &cmd.event.kind)?;
        if !self
            .dependencies
            .deliveries
            .reserve_delivery(&delivery)
            .await?
        {
            return Ok(DispatchInternalAutomationResult {
                duplicate: true,
                rules_triggered: 0,
                rules_failed: 0,
            });
        }
        self.dependencies
            .connections
            .record_delivery_result(connection.id, None)
            .await?;

        let rules = self
            .dependencies
            .rules
            .list_enabled_rules_for_trigger(cmd.team_id, connection.id, &cmd.event.kind)
            .await?;
        let matching_rules: Vec<_> = rules
            .into_iter()
            .filter(|rule| trigger_matches(rule, &cmd.event))
            .collect();
        let executor = AutomationReactionExecutor::new(
            self.dependencies.connections.clone(),
            self.dependencies.credentials.clone(),
            self.dependencies.incidents.clone(),
            self.dependencies.releases.clone(),
            self.dependencies.notifier.clone(),
            self.dependencies.events.clone(),
        );
        let mut rules_triggered = 0;
        let mut rules_failed = 0;
        let mut first_error = None;

        for rule in matching_rules {
            let mut run = AutomationRun::new(delivery.id, rule.id);
            self.dependencies.runs.insert_run(&run).await?;
            match executor.execute(cmd.team_id, &rule, &cmd.event).await {
                Ok(created_incident) => {
                    let incident_id = created_incident.map(|(incident_id, _)| incident_id);
                    run.mark_succeeded(incident_id)?;
                    self.persist_run(&run).await?;
                    rules_triggered += 1;
                    if let Some((incident_id, severity)) = created_incident {
                        self.dependencies
                            .events
                            .publish(DomainEvent::IncidentCreated {
                                team_id: cmd.team_id,
                                incident_id,
                                severity,
                            })
                            .await;
                    }
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleTriggered {
                            team_id: cmd.team_id,
                            service: OPSWARDEN_SERVICE.to_string(),
                            rule_name: rule.name,
                            result: if incident_id.is_some() {
                                AutomationRuleResult::IncidentCreated
                            } else {
                                AutomationRuleResult::ReactionCompleted
                            },
                            incident_id,
                        })
                        .await;
                }
                Err(error) => {
                    let code = error.code();
                    run.mark_failed(code)?;
                    self.persist_run(&run).await?;
                    rules_failed += 1;
                    first_error.get_or_insert(code);
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleFailed {
                            team_id: cmd.team_id,
                            service: OPSWARDEN_SERVICE.to_string(),
                            rule_name: rule.name,
                            error: code.to_string(),
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
        if !self
            .dependencies
            .deliveries
            .update_delivery(&delivery)
            .await?
        {
            return Err(DomainError::InvalidAutomationTransition);
        }
        if first_error.is_some() {
            self.dependencies
                .connections
                .record_delivery_result(connection.id, first_error)
                .await?;
        }

        Ok(DispatchInternalAutomationResult {
            duplicate: false,
            rules_triggered,
            rules_failed,
        })
    }

    async fn persist_run(&self, run: &AutomationRun) -> Result<(), DomainError> {
        if !self.dependencies.runs.update_run(run).await? {
            return Err(DomainError::InvalidAutomationTransition);
        }
        Ok(())
    }
}

pub fn release_created_event(release: &Release) -> ExternalEvent {
    let mut attributes = Map::new();
    attributes.insert("release_id".into(), Value::String(release.id.to_string()));
    attributes.insert("release_title".into(), Value::String(release.title.clone()));
    attributes.insert(
        "release_state".into(),
        Value::String(release.base_state.to_string()),
    );
    ExternalEvent::new(OPSWARDEN_SERVICE, "release_created").with_attributes(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_created_event_contains_only_normalized_release_facts() {
        let release = Release::new(Uuid::new_v4(), "v2.0.0", vec!["build".to_string()]).unwrap();
        let event = release_created_event(&release);
        assert_eq!(event.service, "opswarden");
        assert_eq!(event.kind, "release_created");
        assert_eq!(event.attributes["release_id"], release.id.to_string());
        assert_eq!(event.attributes["release_title"], "v2.0.0");
        assert_eq!(event.attributes["release_state"], "created");
    }
}
