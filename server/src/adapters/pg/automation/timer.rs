// PostgreSQL-backed Timer schedule projection and occurrence claims.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::automation_catalog::TIMER_SERVICE;
use crate::domain::automation_config::{AutomationRun, AutomationRunStatus, WebhookDeliveryStatus};
use crate::domain::automation_timer::{ClaimedTimerOccurrence, TimerSchedule};
use crate::domain::error::DomainError;
use crate::ports::AutomationTimerRepo;

#[path = "timer_rows.rs"]
mod timer_rows;
#[path = "timer_ops.rs"]
mod timer_ops;

use timer_rows::{stored_schedule, DueScheduleRow, UnstartedClaimRow};

pub struct PgAutomationTimerRepo {
    pool: PgPool,
}

impl PgAutomationTimerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AutomationTimerRepo for PgAutomationTimerRepo {
    async fn upsert_schedule(
        &self,
        rule_id: Uuid,
        schedule: &TimerSchedule,
        next_run_at: DateTime<Utc>,
        rule_updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let (local_time, interval_minutes) = match schedule {
            TimerSchedule::DailyAt { time, .. } => (Some(*time), None),
            TimerSchedule::EveryMinutes { minutes, .. } => (None, Some(i32::from(*minutes))),
        };
        let result = sqlx::query(
            r#"
            INSERT INTO automation_timer_schedules (
                rule_id, schedule_kind, timezone, local_time,
                interval_minutes, next_run_at, rule_updated_at, updated_at
            )
            SELECT r.id, $2, $3, $4, $5, $6, $7, now()
            FROM automation_rules r
            JOIN service_connections c
              ON c.id = r.trigger_connection_id
             AND c.team_id = r.team_id
            WHERE r.id = $1
              AND r.enabled
              AND r.updated_at = $7
              AND r.trigger_kind = $2
              AND c.service = $8
            ON CONFLICT (rule_id) DO UPDATE
            SET schedule_kind = excluded.schedule_kind,
                timezone = excluded.timezone,
                local_time = excluded.local_time,
                interval_minutes = excluded.interval_minutes,
                next_run_at = excluded.next_run_at,
                rule_updated_at = excluded.rule_updated_at,
                last_claimed_at = null,
                updated_at = now()
            "#,
        )
        .bind(rule_id)
        .bind(schedule.kind())
        .bind(schedule.timezone().to_string())
        .bind(local_time)
        .bind(interval_minutes)
        .bind(next_run_at)
        .bind(rule_updated_at)
        .bind(TIMER_SERVICE)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn delete_schedule(&self, rule_id: Uuid) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM automation_timer_schedules WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<ClaimedTimerOccurrence>, DomainError> {
        self.claim_due_impl(now).await
    }

    async fn start_execution(
        &self,
        claim: &ClaimedTimerOccurrence,
        run: &AutomationRun,
    ) -> Result<bool, DomainError> {
        self.start_execution_impl(claim, run).await
    }

    async fn list_unstarted_claims(
        &self,
        claimed_before: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<ClaimedTimerOccurrence>, DomainError> {
        self.list_unstarted_claims_impl(claimed_before, limit).await
    }

    async fn finish_execution(
        &self,
        claim: &ClaimedTimerOccurrence,
        run: &AutomationRun,
    ) -> Result<bool, DomainError> {
        if run.delivery_id != claim.delivery_id
            || run.rule_id != Some(claim.rule_id)
            || !matches!(
                run.status,
                AutomationRunStatus::Succeeded | AutomationRunStatus::Failed
            )
            || run.finished_at.is_none()
        {
            return Err(DomainError::InvalidAutomationRun);
        }
        let delivery_status = match run.status {
            AutomationRunStatus::Succeeded => WebhookDeliveryStatus::Processed,
            AutomationRunStatus::Failed => WebhookDeliveryStatus::Failed,
            _ => unreachable!(),
        };
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let updated_run = sqlx::query(
            r#"
            UPDATE automation_runs
            SET status = $4, incident_id = $5, error_code = $6, finished_at = $7
            WHERE id = $1 AND delivery_id = $2 AND rule_id = $3
              AND status = 'running'
            "#,
        )
        .bind(run.id)
        .bind(claim.delivery_id)
        .bind(claim.rule_id)
        .bind(run.status.to_string())
        .bind(run.incident_id)
        .bind(&run.error_code)
        .bind(run.finished_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            == 1;
        if !updated_run {
            transaction
                .rollback()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(false);
        }
        let updated_delivery = sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $2, error_code = $3
            WHERE id = $1 AND connection_id = $4 AND status = 'received'
            "#,
        )
        .bind(claim.delivery_id)
        .bind(delivery_status.to_string())
        .bind(&run.error_code)
        .bind(claim.connection_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            == 1;
        if !updated_delivery {
            transaction
                .rollback()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(false);
        }
        let updated_connection = sqlx::query(
            r#"
            UPDATE service_connections
            SET verified_at = coalesce(verified_at, now()),
                last_delivery_at = now(), last_error_code = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(claim.connection_id)
        .bind(&run.error_code)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            == 1;
        if !updated_connection {
            transaction
                .rollback()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(false);
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(true)
    }

    async fn abandon_claim(
        &self,
        claim: &ClaimedTimerOccurrence,
        finished_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let changed = sqlx::query(
            r#"
            UPDATE automation_timer_occurrences
            SET execution_started_at = $3
            WHERE rule_id = $1
              AND scheduled_for = $2
              AND delivery_id = $4
              AND execution_started_at IS NULL
            "#,
        )
        .bind(claim.rule_id)
        .bind(claim.scheduled_for)
        .bind(finished_at)
        .bind(claim.delivery_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            == 1;
        if !changed {
            transaction
                .commit()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(false);
        }
        sqlx::query(
            r#"
            INSERT INTO automation_runs (
                id, delivery_id, rule_id, status, incident_id, error_code,
                started_at, finished_at
            )
            VALUES ($1, $2, $3, 'skipped', null, null, $4, $4)
            ON CONFLICT (delivery_id, rule_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(claim.delivery_id)
        .bind(claim.rule_id)
        .bind(finished_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        sqlx::query(
            "UPDATE webhook_deliveries SET status = 'ignored' WHERE id = $1 AND status = 'received'",
        )
        .bind(claim.delivery_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(true)
    }

    async fn finalize_stale_runs(
        &self,
        started_before: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Result<u64, DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let delivery_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH stale AS (
                UPDATE automation_runs r
                SET status = 'failed',
                    error_code = 'timer_worker_interrupted',
                    finished_at = $2
                FROM automation_timer_occurrences o
                WHERE r.delivery_id = o.delivery_id
                  AND r.status = 'running'
                  AND r.started_at <= $1
                RETURNING r.delivery_id
            )
            UPDATE webhook_deliveries d
            SET status = 'failed', error_code = 'timer_worker_interrupted'
            FROM stale
            WHERE d.id = stale.delivery_id
              AND d.status = 'received'
            RETURNING d.id
            "#,
        )
        .bind(started_before)
        .bind(finished_at)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if !delivery_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE service_connections
                SET verified_at = coalesce(verified_at, $2),
                    last_delivery_at = $2,
                    last_error_code = 'timer_worker_interrupted',
                    updated_at = $2
                WHERE id IN (
                    SELECT connection_id FROM webhook_deliveries WHERE id = ANY($1)
                )
                "#,
            )
            .bind(&delivery_ids)
            .bind(finished_at)
            .execute(&mut *transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(delivery_ids.len() as u64)
    }
}

#[cfg(test)]
#[path = "timer_tests.rs"]
mod tests;
