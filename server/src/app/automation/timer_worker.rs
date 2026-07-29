use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::domain::automation_config::{AutomationRun, WebhookDelivery, WebhookDeliveryStatus};
use crate::domain::automation_timer::{ClaimedTimerOccurrence, TimerSchedule, TIMER_SERVICE};
use crate::domain::error::DomainError;
use crate::domain::event::{AutomationRuleResult, DomainEvent};
use crate::ports::{
    AutomationRuleRepo, AutomationRunRepo, AutomationTimerRepo, ConnectionCredentialVault,
    EventPublisher, IncidentRepo, Notifier, ReleaseRepo, ServiceConnectionRepo,
    WebhookDeliveryRepo,
};

use super::AutomationReactionExecutor;

pub const TIMER_BATCH_SIZE: usize = 32;
const UNSTARTED_CLAIM_GRACE_SECONDS: i64 = 30;
const STALE_RUN_AFTER_MINUTES: i64 = 5;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TimerTickResult {
    pub claimed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TimerReconcileResult {
    pub recovered: usize,
    pub stale_runs_finalized: u64,
}

pub struct TimerWorkerDependencies {
    pub timers: Arc<dyn AutomationTimerRepo>,
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

pub struct TimerWorker {
    dependencies: TimerWorkerDependencies,
}

impl TimerWorker {
    pub fn new(dependencies: TimerWorkerDependencies) -> Self {
        Self { dependencies }
    }

    pub async fn tick(&self, now: DateTime<Utc>) -> Result<TimerTickResult, DomainError> {
        let mut result = TimerTickResult::default();
        for _ in 0..TIMER_BATCH_SIZE {
            let Some(claim) = self.dependencies.timers.claim_due(now).await? else {
                break;
            };
            result.claimed += 1;
            match self.execute_claim(&claim, now).await? {
                ClaimResult::Succeeded => result.succeeded += 1,
                ClaimResult::Failed => result.failed += 1,
                ClaimResult::Skipped => result.skipped += 1,
            }
        }
        Ok(result)
    }

    pub async fn reconcile(&self, now: DateTime<Utc>) -> Result<TimerReconcileResult, DomainError> {
        let stale_runs_finalized = self
            .dependencies
            .timers
            .finalize_stale_runs(now - Duration::minutes(STALE_RUN_AFTER_MINUTES), now)
            .await?;
        let claims = self
            .dependencies
            .timers
            .list_unstarted_claims(
                now - Duration::seconds(UNSTARTED_CLAIM_GRACE_SECONDS),
                TIMER_BATCH_SIZE as u32,
            )
            .await?;
        let mut recovered = 0;
        for claim in claims {
            self.execute_claim(&claim, now).await?;
            recovered += 1;
        }
        Ok(TimerReconcileResult {
            recovered,
            stale_runs_finalized,
        })
    }

    async fn execute_claim(
        &self,
        claim: &ClaimedTimerOccurrence,
        now: DateTime<Utc>,
    ) -> Result<ClaimResult, DomainError> {
        let rule = self
            .dependencies
            .rules
            .find_rule_for_team(claim.team_id, claim.rule_id)
            .await?;
        let valid_rule = rule.as_ref().is_some_and(|rule| {
            rule.enabled
                && rule.trigger_connection_id == claim.connection_id
                && rule.updated_at == claim.rule_updated_at
                && TimerSchedule::from_config(&rule.trigger_kind, &rule.trigger_config)
                    .is_ok_and(|schedule| schedule == claim.schedule)
        });
        let Some(rule) = rule.filter(|_| valid_rule) else {
            self.dependencies.timers.abandon_claim(claim, now).await?;
            return Ok(ClaimResult::Skipped);
        };

        let mut run = AutomationRun::new(claim.delivery_id, claim.rule_id);
        if !self
            .dependencies
            .timers
            .start_execution(claim, &run)
            .await?
        {
            return Ok(ClaimResult::Skipped);
        }

        let event = claim
            .schedule
            .occurrence(claim.rule_id, claim.scheduled_for)
            .event;
        let executor = AutomationReactionExecutor::new(
            self.dependencies.connections.clone(),
            self.dependencies.credentials.clone(),
            self.dependencies.incidents.clone(),
            self.dependencies.releases.clone(),
            self.dependencies.notifier.clone(),
            self.dependencies.events.clone(),
        );
        match executor.execute(claim.team_id, &rule, &event).await {
            Ok(created_incident) => {
                let incident_id = created_incident.map(|(id, _)| id);
                run.mark_succeeded(incident_id)?;
                self.persist_run(&run).await?;
                self.finish_delivery(claim, WebhookDeliveryStatus::Processed, None)
                    .await?;
                if let Some((incident_id, severity)) = created_incident {
                    self.dependencies
                        .events
                        .publish(DomainEvent::IncidentCreated {
                            team_id: claim.team_id,
                            incident_id,
                            severity,
                        })
                        .await;
                }
                self.dependencies
                    .events
                    .publish(DomainEvent::RuleTriggered {
                        team_id: claim.team_id,
                        service: TIMER_SERVICE.to_string(),
                        rule_name: rule.name,
                        result: if incident_id.is_some() {
                            AutomationRuleResult::IncidentCreated
                        } else {
                            AutomationRuleResult::ReactionCompleted
                        },
                        incident_id,
                    })
                    .await;
                Ok(ClaimResult::Succeeded)
            }
            Err(error) => {
                let code = error.code();
                run.mark_failed(code)?;
                self.persist_run(&run).await?;
                self.finish_delivery(claim, WebhookDeliveryStatus::Failed, Some(code))
                    .await?;
                self.dependencies
                    .events
                    .publish(DomainEvent::RuleFailed {
                        team_id: claim.team_id,
                        service: TIMER_SERVICE.to_string(),
                        rule_name: rule.name,
                        error: code.to_string(),
                    })
                    .await;
                Ok(ClaimResult::Failed)
            }
        }
    }

    async fn persist_run(&self, run: &AutomationRun) -> Result<(), DomainError> {
        if !self.dependencies.runs.update_run(run).await? {
            return Err(DomainError::InvalidAutomationTransition);
        }
        Ok(())
    }

    async fn finish_delivery(
        &self,
        claim: &ClaimedTimerOccurrence,
        status: WebhookDeliveryStatus,
        error_code: Option<&str>,
    ) -> Result<(), DomainError> {
        let delivery = WebhookDelivery {
            id: claim.delivery_id,
            connection_id: claim.connection_id,
            provider_delivery_id: claim.provider_delivery_id(),
            provider_event: claim.schedule.kind().to_string(),
            status,
            error_code: error_code.map(str::to_string),
            received_at: claim.claimed_at,
        };
        if !self
            .dependencies
            .deliveries
            .update_delivery(&delivery)
            .await?
        {
            return Err(DomainError::InvalidAutomationTransition);
        }
        self.dependencies
            .connections
            .record_delivery_result(claim.connection_id, error_code)
            .await?;
        Ok(())
    }
}

enum ClaimResult {
    Succeeded,
    Failed,
    Skipped,
}
