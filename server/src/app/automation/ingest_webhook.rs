// --- server/src/app/automation/ingest_webhook.rs ---
//
// The hook engine use-case: authenticate an inbound webhook, decode it into a
// domain event, evaluate the configured rules and run their reactions. A webhook
// is authorized by its HMAC signature (the shared secret), not by a user/JWT, so
// reactions run as system actions and bypass the per-user RBAC of the REST paths.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::automation::{evaluate, Reaction};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::Incident;
use crate::ports::{
    EventPublisher, IncidentRepo, Notifier, RuleRepo, SecretVault, WebhookParser, WebhookVerifier,
};

pub struct IngestWebhookCommand {
    pub service: String,
    pub signature: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IngestWebhookResult {
    /// Number of rules whose reaction fired (0 when the payload matched no rule
    /// or is one we don't act on).
    pub rules_triggered: usize,
}

pub struct IngestWebhookUseCase {
    vault: Arc<dyn SecretVault>,
    verifier: Arc<dyn WebhookVerifier>,
    parser: Arc<dyn WebhookParser>,
    rules: Arc<dyn RuleRepo>,
    incidents: Arc<dyn IncidentRepo>,
    notifier: Arc<dyn Notifier>,
    events: Arc<dyn EventPublisher>,
}

impl IngestWebhookUseCase {
    pub fn new(
        vault: Arc<dyn SecretVault>,
        verifier: Arc<dyn WebhookVerifier>,
        parser: Arc<dyn WebhookParser>,
        rules: Arc<dyn RuleRepo>,
        incidents: Arc<dyn IncidentRepo>,
        notifier: Arc<dyn Notifier>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            vault,
            verifier,
            parser,
            rules,
            incidents,
            notifier,
            events,
        }
    }

    pub async fn ingest(
        &self,
        cmd: IngestWebhookCommand,
    ) -> Result<IngestWebhookResult, DomainError> {
        // 1. Authenticate: the per-service secret authorizes the call.
        let secret = self
            .vault
            .reveal(&cmd.service)
            .await?
            .ok_or(DomainError::UnknownService)?;

        let signature = cmd.signature.as_deref().unwrap_or_default();
        if !self.verifier.verify(&secret, &cmd.body, signature) {
            return Err(DomainError::InvalidSignature);
        }

        // 2. Decode: a signed-but-unrecognized payload is acknowledged, not failed.
        let Some(event) = self.parser.parse(&cmd.service, &cmd.body) else {
            return Ok(IngestWebhookResult { rules_triggered: 0 });
        };

        // 3. Evaluate + react. A single reaction failure is reported via
        //    `rule_failed` and never aborts the others or the acknowledgement.
        let rules = self.rules.list_rules().await?;
        let mut rules_triggered = 0;
        for rule in evaluate(&rules, &event) {
            match self.run_reaction(&rule.reaction).await {
                Ok(incident_id) => {
                    rules_triggered += 1;
                    self.events
                        .publish(DomainEvent::RuleTriggered {
                            team_id: rule.reaction.team_id(),
                            service: event.service.clone(),
                            rule: rule.name.clone(),
                            incident_id,
                        })
                        .await;
                }
                Err(err) => {
                    self.events
                        .publish(DomainEvent::RuleFailed {
                            team_id: rule.reaction.team_id(),
                            service: event.service.clone(),
                            rule: rule.name.clone(),
                            reason: err.to_string(),
                        })
                        .await;
                }
            }
        }

        Ok(IngestWebhookResult { rules_triggered })
    }

    /// Run a reaction as a system action (authority = the verified webhook).
    /// Returns the opened incident id for `CreateIncident`, `None` for
    /// side-effect reactions like `Notify`.
    async fn run_reaction(&self, reaction: &Reaction) -> Result<Option<Uuid>, DomainError> {
        match reaction {
            Reaction::CreateIncident {
                team_id,
                severity,
                title,
            } => {
                let incident = Incident::new(*team_id, title.clone(), *severity)?;
                self.incidents.save_incident(&incident).await?;
                Ok(Some(incident.id))
            }
            Reaction::Notify { url, message, .. } => {
                self.notifier.notify(url, message).await?;
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;

    use crate::adapters::automation::StaticRuleRepo;
    use crate::adapters::crypto::hmac::{hmac_sha256, HmacSha256Verifier};
    use crate::adapters::webhook::github::GithubParser;
    use crate::app::incident::tests::{MockEventPublisher, MockIncidentRepo};
    use crate::domain::automation::Rule;
    use crate::domain::incident::Severity;
    use std::sync::Mutex;

    const SECRET: &str = "webhook-shared-secret";
    const FAILED_RUN: &[u8] = br#"{"workflow_run":{"conclusion":"failure"}}"#;
    const PASSED_RUN: &[u8] = br#"{"workflow_run":{"conclusion":"success"}}"#;

    struct MockVault {
        secret: Option<String>,
    }

    #[async_trait]
    impl SecretVault for MockVault {
        async fn store(&self, _service: &str, _secret: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn reveal(&self, _service: &str) -> Result<Option<String>, DomainError> {
            Ok(self.secret.clone())
        }
        async fn delete(&self, _service: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockNotifier {
        calls: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl Notifier for MockNotifier {
        async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError> {
            self.calls
                .lock()
                .unwrap()
                .push((url.to_string(), message.to_string()));
            Ok(())
        }
    }

    fn signature_for(body: &[u8]) -> String {
        format!(
            "sha256={}",
            hex::encode(hmac_sha256(SECRET.as_bytes(), body))
        )
    }

    fn use_case(
        vault_secret: Option<&str>,
        team_id: Uuid,
        incidents: Arc<MockIncidentRepo>,
        events: Arc<MockEventPublisher>,
    ) -> IngestWebhookUseCase {
        IngestWebhookUseCase::new(
            Arc::new(MockVault {
                secret: vault_secret.map(str::to_string),
            }),
            Arc::new(HmacSha256Verifier),
            Arc::new(GithubParser),
            Arc::new(StaticRuleRepo::github_ci_to_incident(team_id)),
            incidents,
            Arc::new(MockNotifier::default()),
            events,
        )
    }

    #[tokio::test]
    async fn valid_signed_ci_failure_opens_a_high_incident() {
        let team_id = Uuid::new_v4();
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(Some(SECRET), team_id, incidents.clone(), events.clone());

        let result = uc
            .ingest(IngestWebhookCommand {
                service: "github".to_string(),
                signature: Some(signature_for(FAILED_RUN)),
                body: FAILED_RUN.to_vec(),
            })
            .await
            .unwrap();

        assert_eq!(result.rules_triggered, 1);
        let saved = incidents.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].team_id, team_id);
        assert_eq!(saved[0].severity, Severity::High);

        let published = events.published.lock().unwrap();
        assert!(matches!(
            published.as_slice(),
            [DomainEvent::RuleTriggered { .. }]
        ));
    }

    #[tokio::test]
    async fn invalid_signature_is_rejected_and_creates_nothing() {
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(
            Some(SECRET),
            Uuid::new_v4(),
            incidents.clone(),
            events.clone(),
        );

        let err = uc
            .ingest(IngestWebhookCommand {
                service: "github".to_string(),
                signature: Some("sha256=deadbeef".to_string()),
                body: FAILED_RUN.to_vec(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::InvalidSignature);
        assert!(incidents.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unknown_service_has_no_secret() {
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(None, Uuid::new_v4(), incidents.clone(), events);

        let err = uc
            .ingest(IngestWebhookCommand {
                service: "github".to_string(),
                signature: Some(signature_for(FAILED_RUN)),
                body: FAILED_RUN.to_vec(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::UnknownService);
        assert!(incidents.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn signed_but_non_failing_payload_triggers_no_rule() {
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(Some(SECRET), Uuid::new_v4(), incidents.clone(), events);

        let result = uc
            .ingest(IngestWebhookCommand {
                service: "github".to_string(),
                signature: Some(signature_for(PASSED_RUN)),
                body: PASSED_RUN.to_vec(),
            })
            .await
            .unwrap();

        assert_eq!(result.rules_triggered, 0);
        assert!(incidents.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn signed_ci_failure_fires_a_notify_reaction() {
        let team_id = Uuid::new_v4();
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let notifier = Arc::new(MockNotifier::default());

        let notify_rule = Rule {
            name: "notify".to_string(),
            on_service: "github".to_string(),
            on_kind: "ci_failed".to_string(),
            reaction: Reaction::Notify {
                team_id,
                url: "https://hooks.slack.com/services/x".to_string(),
                message: "CI failed".to_string(),
            },
        };

        let uc = IngestWebhookUseCase::new(
            Arc::new(MockVault {
                secret: Some(SECRET.to_string()),
            }),
            Arc::new(HmacSha256Verifier),
            Arc::new(GithubParser),
            Arc::new(StaticRuleRepo::new(vec![notify_rule])),
            incidents.clone(),
            notifier.clone(),
            events.clone(),
        );

        let result = uc
            .ingest(IngestWebhookCommand {
                service: "github".to_string(),
                signature: Some(signature_for(FAILED_RUN)),
                body: FAILED_RUN.to_vec(),
            })
            .await
            .unwrap();

        assert_eq!(result.rules_triggered, 1);
        // Notify fires the outbound call but opens no incident.
        assert_eq!(notifier.calls.lock().unwrap().len(), 1);
        assert!(incidents.saved.lock().unwrap().is_empty());

        let published = events.published.lock().unwrap();
        assert!(matches!(
            published.as_slice(),
            [DomainEvent::RuleTriggered {
                incident_id: None,
                ..
            }]
        ));
    }
}
