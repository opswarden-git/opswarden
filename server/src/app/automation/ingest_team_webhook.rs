use std::sync::Arc;

use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_catalog::{service, WebhookAuthentication};
use crate::domain::automation_config::{
    AutomationRule, AutomationRun, AutomationRunStatus, CredentialKind, WebhookDelivery,
};
use crate::domain::error::DomainError;
use crate::domain::event::AutomationRuleResult;
use crate::domain::event::DomainEvent;
use crate::ports::{
    AutomationRuleRepo, AutomationRunRepo, AutomationRunReservation, ConnectionCredentialVault,
    EmailSender, EventPublisher, IncidentRepo, Notifier, ReleaseRepo, ServiceConnectionRepo,
    WebhookDeliveryRepo, WebhookJob, WebhookJobRepo, WebhookParser, WebhookVerifier,
};

use super::reaction_executor::AutomationReactionExecutor;

pub struct IngestTeamWebhookCommand {
    pub connection_id: Uuid,
    pub expected_service: &'static str,
    pub provider_delivery_id: String,
    pub provider_event: String,
    pub signature: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IngestTeamWebhookResult {
    pub duplicate: bool,
    pub ignored: bool,
    pub rules_triggered: usize,
    pub rules_failed: usize,
}

#[derive(Clone)]
pub struct TeamWebhookDependencies {
    pub connections: Arc<dyn ServiceConnectionRepo>,
    pub credentials: Arc<dyn ConnectionCredentialVault>,
    pub verifier: Arc<dyn WebhookVerifier>,
    pub parser: Arc<dyn WebhookParser>,
    pub deliveries: Arc<dyn WebhookDeliveryRepo>,
    pub rules: Arc<dyn AutomationRuleRepo>,
    pub runs: Arc<dyn AutomationRunRepo>,
    pub incidents: Arc<dyn IncidentRepo>,
    pub releases: Arc<dyn ReleaseRepo>,
    pub notifier: Arc<dyn Notifier>,
    pub events: Arc<dyn EventPublisher>,
    pub email_sender: Arc<dyn EmailSender>,
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
        let connection = self.authenticate(&cmd).await?;
        self.process(
            connection,
            cmd.provider_delivery_id,
            cmd.provider_event,
            cmd.body,
        )
        .await
    }

    pub async fn process_job(
        &self,
        job: WebhookJob,
    ) -> Result<IngestTeamWebhookResult, DomainError> {
        let connection = self
            .dependencies
            .connections
            .find_connection_by_id(job.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if connection.service != job.expected_service {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        self.process(
            connection,
            job.provider_delivery_id,
            job.provider_event,
            job.body,
        )
        .await
    }

    async fn authenticate(
        &self,
        cmd: &IngestTeamWebhookCommand,
    ) -> Result<crate::domain::automation_config::ServiceConnection, DomainError> {
        let connection = self
            .dependencies
            .connections
            .find_connection_by_id(cmd.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if connection.service != cmd.expected_service {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        let secret = self
            .dependencies
            .credentials
            .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
            .await?
            .ok_or(DomainError::UnknownService)?;
        let authentication = cmd.signature.as_deref().unwrap_or_default();
        let authentication_mode = service(&connection.service)
            .and_then(|definition| definition.connection)
            .and_then(|connection| connection.webhook_authentication)
            .ok_or(DomainError::UnknownService)?;
        let authenticated = match authentication_mode {
            WebhookAuthentication::Signature => {
                self.dependencies
                    .verifier
                    .verify(&secret, &cmd.body, authentication)
            }
            WebhookAuthentication::Token => self
                .dependencies
                .verifier
                .verify_token(&secret, authentication),
        };
        if !authenticated {
            return Err(DomainError::InvalidSignature);
        }
        Ok(connection)
    }

    async fn process(
        &self,
        connection: crate::domain::automation_config::ServiceConnection,
        provider_delivery_id: String,
        provider_event: String,
        body: Vec<u8>,
    ) -> Result<IngestTeamWebhookResult, DomainError> {
        let mut delivery =
            WebhookDelivery::new(connection.id, provider_delivery_id, provider_event)?;
        let Some(claim) = self
            .dependencies
            .deliveries
            .claim_delivery(&delivery)
            .await?
        else {
            return Ok(IngestTeamWebhookResult {
                duplicate: true,
                ignored: false,
                rules_triggered: 0,
                rules_failed: 0,
            });
        };
        delivery.id = claim.delivery_id;
        // A new, correctly signed provider delivery proves the connection.
        // Duplicate retries deliberately keep the original delivery health.
        self.dependencies
            .connections
            .record_delivery_result(connection.id, None)
            .await?;

        let Some(event) =
            self.dependencies
                .parser
                .parse(&connection.service, &delivery.provider_event, &body)
        else {
            delivery.mark_ignored()?;
            self.persist_delivery(&delivery, claim).await?;
            return Ok(IngestTeamWebhookResult {
                duplicate: false,
                ignored: true,
                rules_triggered: 0,
                rules_failed: 0,
            });
        };

        self.dependencies
            .runs
            .interrupt_running_for_delivery(claim)
            .await?;

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
        let mut first_error_code: Option<String> = None;
        let executor = AutomationReactionExecutor::new(
            self.dependencies.connections.clone(),
            self.dependencies.credentials.clone(),
            self.dependencies.incidents.clone(),
            self.dependencies.releases.clone(),
            self.dependencies.notifier.clone(),
            self.dependencies.events.clone(),
            self.dependencies.email_sender.clone(),
        );
        for rule in matching_rules {
            let candidate = AutomationRun::new(delivery.id, rule.id);
            let mut run = match self
                .dependencies
                .runs
                .reserve_run(&candidate, claim)
                .await?
            {
                AutomationRunReservation::New(run) => run,
                AutomationRunReservation::Existing(run) => {
                    match run.status {
                        AutomationRunStatus::Succeeded => rules_triggered += 1,
                        AutomationRunStatus::Failed => {
                            rules_failed += 1;
                            if let Some(error_code) = run.error_code.as_deref() {
                                first_error_code.get_or_insert_with(|| error_code.to_string());
                            }
                        }
                        AutomationRunStatus::Skipped => {}
                        AutomationRunStatus::Running => {
                            return Err(DomainError::InvalidAutomationTransition);
                        }
                    }
                    continue;
                }
            };
            match executor.execute(connection.team_id, &rule, &event).await {
                Ok(created_incident) => {
                    let incident_id = created_incident.map(|(incident_id, _)| incident_id);
                    run.mark_succeeded(incident_id)?;
                    self.persist_run(&run).await?;
                    rules_triggered += 1;
                    if let Some((incident_id, severity)) = created_incident {
                        self.dependencies
                            .events
                            .publish(DomainEvent::IncidentCreated {
                                team_id: connection.team_id,
                                incident_id,
                                severity,
                            })
                            .await;
                    }
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleTriggered {
                            team_id: connection.team_id,
                            service: connection.service.clone(),
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
                    let error_code = error.code();
                    run.mark_failed(error_code)?;
                    self.persist_run(&run).await?;
                    rules_failed += 1;
                    first_error_code.get_or_insert_with(|| error_code.to_string());
                    self.dependencies
                        .events
                        .publish(DomainEvent::RuleFailed {
                            team_id: connection.team_id,
                            service: connection.service.clone(),
                            rule_name: rule.name,
                            error: error_code.to_string(),
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
        self.persist_delivery(&delivery, claim).await?;
        if first_error_code.is_some() {
            self.dependencies
                .connections
                .record_delivery_result(connection.id, first_error_code.as_deref())
                .await?;
        }

        Ok(IngestTeamWebhookResult {
            duplicate: false,
            ignored: rules_triggered == 0 && rules_failed == 0,
            rules_triggered,
            rules_failed,
        })
    }

    async fn persist_delivery(
        &self,
        delivery: &WebhookDelivery,
        claim: crate::ports::WebhookDeliveryClaim,
    ) -> Result<(), DomainError> {
        if !self
            .dependencies
            .deliveries
            .complete_claimed_delivery(delivery, claim)
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

#[async_trait::async_trait]
pub trait TeamWebhookIngress: Send + Sync {
    async fn accept(
        &self,
        cmd: IngestTeamWebhookCommand,
    ) -> Result<IngestTeamWebhookResult, DomainError>;
    async fn accept_batch(
        &self,
        commands: Vec<IngestTeamWebhookCommand>,
    ) -> Result<Vec<IngestTeamWebhookResult>, DomainError>;
}

#[async_trait::async_trait]
impl TeamWebhookIngress for IngestTeamWebhookUseCase {
    async fn accept(
        &self,
        cmd: IngestTeamWebhookCommand,
    ) -> Result<IngestTeamWebhookResult, DomainError> {
        self.ingest(cmd).await
    }

    async fn accept_batch(
        &self,
        commands: Vec<IngestTeamWebhookCommand>,
    ) -> Result<Vec<IngestTeamWebhookResult>, DomainError> {
        let mut results = Vec::with_capacity(commands.len());
        for command in commands {
            results.push(self.ingest(command).await?);
        }
        Ok(results)
    }
}

pub struct DurableTeamWebhookIngress {
    use_case: IngestTeamWebhookUseCase,
    jobs: Arc<dyn WebhookJobRepo>,
}

impl DurableTeamWebhookIngress {
    pub fn new(dependencies: TeamWebhookDependencies, jobs: Arc<dyn WebhookJobRepo>) -> Self {
        Self {
            use_case: IngestTeamWebhookUseCase::new(dependencies),
            jobs,
        }
    }
}

#[async_trait::async_trait]
impl TeamWebhookIngress for DurableTeamWebhookIngress {
    async fn accept(
        &self,
        cmd: IngestTeamWebhookCommand,
    ) -> Result<IngestTeamWebhookResult, DomainError> {
        self.accept_batch(vec![cmd])
            .await?
            .into_iter()
            .next()
            .ok_or(DomainError::Storage)
    }

    async fn accept_batch(
        &self,
        commands: Vec<IngestTeamWebhookCommand>,
    ) -> Result<Vec<IngestTeamWebhookResult>, DomainError> {
        let mut jobs = Vec::with_capacity(commands.len());
        for cmd in commands {
            self.use_case.authenticate(&cmd).await?;
            let delivery = WebhookDelivery::new(
                cmd.connection_id,
                cmd.provider_delivery_id,
                cmd.provider_event,
            )?;
            jobs.push(WebhookJob {
                id: Uuid::new_v4(),
                connection_id: cmd.connection_id,
                expected_service: cmd.expected_service.to_string(),
                provider_delivery_id: delivery.provider_delivery_id,
                provider_event: delivery.provider_event,
                body: cmd.body,
            });
        }
        Ok(self
            .jobs
            .enqueue_batch(&jobs)
            .await?
            .into_iter()
            .map(|accepted| IngestTeamWebhookResult {
                duplicate: !accepted,
                ignored: false,
                rules_triggered: 0,
                rules_failed: 0,
            })
            .collect())
    }
}

pub(crate) fn trigger_matches(rule: &AutomationRule, event: &ExternalEvent) -> bool {
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
            "create_incident",
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
