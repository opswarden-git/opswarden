use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::domain::automation_catalog::TIMER_SERVICE;
use crate::domain::automation_config::AutomationRun;
use crate::domain::automation_timer::{ClaimedTimerOccurrence, TimerSchedule};
use crate::domain::error::DomainError;
use crate::domain::event::{AutomationRuleResult, DomainEvent};
use crate::ports::{
    AutomationRuleRepo, AutomationTimerRepo, ConnectionCredentialVault, EmailSender,
    EventPublisher, IncidentRepo, Notifier, ReleaseRepo, ServiceConnectionRepo,
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
    pub retried: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TimerReconcileResult {
    pub recovered: usize,
    pub retried: usize,
    pub stale_runs_finalized: u64,
}

pub struct TimerWorkerDependencies {
    pub timers: Arc<dyn AutomationTimerRepo>,
    pub connections: Arc<dyn ServiceConnectionRepo>,
    pub credentials: Arc<dyn ConnectionCredentialVault>,
    pub rules: Arc<dyn AutomationRuleRepo>,
    pub incidents: Arc<dyn IncidentRepo>,
    pub releases: Arc<dyn ReleaseRepo>,
    pub notifier: Arc<dyn Notifier>,
    pub events: Arc<dyn EventPublisher>,
    pub email_sender: Arc<dyn EmailSender>,
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
            match self.execute_claim(&claim, now).await {
                Ok(ClaimResult::Succeeded) => result.succeeded += 1,
                Ok(ClaimResult::Failed) => result.failed += 1,
                Ok(ClaimResult::Skipped) => result.skipped += 1,
                Err(error) => {
                    result.retried += 1;
                    log_deferred_claim(&claim, &error);
                }
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
        let mut retried = 0;
        for claim in claims {
            match self.execute_claim(&claim, now).await {
                Ok(_) => recovered += 1,
                Err(error) => {
                    retried += 1;
                    log_deferred_claim(&claim, &error);
                }
            }
        }
        Ok(TimerReconcileResult {
            recovered,
            retried,
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
            self.dependencies.email_sender.clone(),
        );
        match executor.execute(claim.team_id, &rule, &event).await {
            Ok(created_incident) => {
                let incident_id = created_incident.map(|(id, _)| id);
                run.mark_succeeded(incident_id)?;
                self.finish_execution(claim, &run).await?;
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
                self.finish_execution(claim, &run).await?;
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

    async fn finish_execution(
        &self,
        claim: &ClaimedTimerOccurrence,
        run: &AutomationRun,
    ) -> Result<(), DomainError> {
        if !self
            .dependencies
            .timers
            .finish_execution(claim, run)
            .await?
        {
            return Err(DomainError::InvalidAutomationTransition);
        }
        Ok(())
    }
}

enum ClaimResult {
    Succeeded,
    Failed,
    Skipped,
}

fn log_deferred_claim(claim: &ClaimedTimerOccurrence, error: &DomainError) {
    tracing::error!(
        rule_id = %claim.rule_id,
        delivery_id = %claim.delivery_id,
        error_code = error.code(),
        "timer occurrence deferred"
    );
}
