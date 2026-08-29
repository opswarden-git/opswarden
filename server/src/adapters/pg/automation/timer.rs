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
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let row = sqlx::query_as::<_, DueScheduleRow>(
            r#"
            SELECT s.rule_id,
                   r.team_id,
                   r.trigger_connection_id AS connection_id,
                   s.schedule_kind,
                   s.timezone,
                   s.local_time,
                   s.interval_minutes,
                   s.next_run_at AS scheduled_for,
                   s.rule_updated_at
            FROM automation_timer_schedules s
            JOIN automation_rules r ON r.id = s.rule_id
            JOIN service_connections c
              ON c.id = r.trigger_connection_id
             AND c.team_id = r.team_id
            WHERE s.next_run_at <= $1
              AND s.rule_updated_at = r.updated_at
              AND r.enabled
              AND r.trigger_kind = s.schedule_kind
              AND c.service = $2
            ORDER BY s.next_run_at, s.rule_id
            FOR UPDATE OF s SKIP LOCKED
            LIMIT 1
            "#,
        )
        .bind(now)
        .bind(TIMER_SERVICE)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        let Some(row) = row else {
            transaction
                .commit()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(None);
        };

        let schedule = row.schedule()?;
        let Some(scheduled_for) = schedule.recovery_occurrence(row.scheduled_for, now) else {
            let next_run_at = schedule.next_after(now);
            sqlx::query(
                "UPDATE automation_timer_schedules SET next_run_at = $2, updated_at = $3 WHERE rule_id = $1",
            )
            .bind(row.rule_id)
            .bind(next_run_at)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
            transaction
                .commit()
                .await
                .map_err(|_| DomainError::Storage)?;
            return Ok(None);
        };
        // A delayed worker emits one recovery occurrence, then resumes from
        // the current instant. It never drains an unbounded missed backlog.
        let next_run_at = schedule.next_after(now.max(scheduled_for));
        let delivery_id = Uuid::new_v4();
        let provider_delivery_id = format!("timer:{}:{}", row.rule_id, scheduled_for.timestamp());

        sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (
                id, connection_id, provider_delivery_id, provider_event,
                status, error_code, received_at
            )
            VALUES ($1, $2, $3, $4, $5, null, $6)
            "#,
        )
        .bind(delivery_id)
        .bind(row.connection_id)
        .bind(provider_delivery_id)
        .bind(schedule.kind())
        .bind(WebhookDeliveryStatus::Received.to_string())
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query(
            r#"
            INSERT INTO automation_timer_occurrences (
                rule_id, scheduled_for, delivery_id, schedule_kind, timezone,
                local_time, interval_minutes, rule_updated_at, claimed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(row.rule_id)
        .bind(scheduled_for)
        .bind(delivery_id)
        .bind(schedule.kind())
        .bind(schedule.timezone().to_string())
        .bind(row.local_time)
        .bind(row.interval_minutes)
        .bind(row.rule_updated_at)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query(
            r#"
            UPDATE automation_timer_schedules
            SET next_run_at = $2,
                last_claimed_at = $3,
                updated_at = $3
            WHERE rule_id = $1
            "#,
        )
        .bind(row.rule_id)
        .bind(next_run_at)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;

        Ok(Some(ClaimedTimerOccurrence {
            rule_id: row.rule_id,
            team_id: row.team_id,
            connection_id: row.connection_id,
            delivery_id,
            scheduled_for,
            claimed_at: now,
            rule_updated_at: row.rule_updated_at,
            schedule,
        }))
    }

    async fn start_execution(
        &self,
        claim: &ClaimedTimerOccurrence,
        run: &AutomationRun,
    ) -> Result<bool, DomainError> {
        if run.delivery_id != claim.delivery_id
            || run.rule_id != Some(claim.rule_id)
            || run.status != AutomationRunStatus::Running
            || run.finished_at.is_some()
        {
            return Err(DomainError::InvalidAutomationRun);
        }
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let started = sqlx::query(
            r#"
            UPDATE automation_timer_occurrences o
            SET execution_started_at = $3
            WHERE o.rule_id = $1
              AND o.scheduled_for = $2
              AND o.delivery_id = $4
              AND o.execution_started_at IS NULL
              AND EXISTS (
                  SELECT 1
                  FROM automation_rules r
                  JOIN service_connections c
                    ON c.id = r.trigger_connection_id
                   AND c.team_id = r.team_id
                  WHERE r.id = o.rule_id
                    AND r.team_id = $5
                    AND r.trigger_connection_id = $6
                    AND r.updated_at = $7
                    AND r.enabled
                    AND r.trigger_kind = $8
                    AND c.service = $9
              )
            "#,
        )
        .bind(claim.rule_id)
        .bind(claim.scheduled_for)
        .bind(run.started_at)
        .bind(claim.delivery_id)
        .bind(claim.team_id)
        .bind(claim.connection_id)
        .bind(claim.rule_updated_at)
        .bind(claim.schedule.kind())
        .bind(TIMER_SERVICE)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            == 1;
        if !started {
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
            VALUES ($1, $2, $3, $4, null, null, $5, null)
            "#,
        )
        .bind(run.id)
        .bind(run.delivery_id)
        .bind(claim.rule_id)
        .bind(run.status.to_string())
        .bind(run.started_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(true)
    }

    async fn list_unstarted_claims(
        &self,
        claimed_before: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<ClaimedTimerOccurrence>, DomainError> {
        let rows = sqlx::query_as::<_, UnstartedClaimRow>(
            r#"
            SELECT o.rule_id,
                   c.team_id,
                   d.connection_id,
                   o.delivery_id,
                   o.schedule_kind,
                   o.timezone,
                   o.local_time,
                   o.interval_minutes,
                   o.scheduled_for,
                   o.claimed_at,
                   o.rule_updated_at
            FROM automation_timer_occurrences o
            JOIN webhook_deliveries d ON d.id = o.delivery_id
            JOIN service_connections c ON c.id = d.connection_id
            WHERE o.execution_started_at IS NULL
              AND o.claimed_at <= $1
              AND d.status = 'received'
            ORDER BY o.claimed_at, o.rule_id
            LIMIT $2
            "#,
        )
        .bind(claimed_before)
        .bind(i64::from(limit.clamp(1, 200)))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        rows.into_iter()
            .map(|row| {
                Ok(ClaimedTimerOccurrence {
                    rule_id: row.rule_id,
                    team_id: row.team_id,
                    connection_id: row.connection_id,
                    delivery_id: row.delivery_id,
                    scheduled_for: row.scheduled_for,
                    claimed_at: row.claimed_at,
                    rule_updated_at: row.rule_updated_at,
                    schedule: stored_schedule(
                        &row.schedule_kind,
                        &row.timezone,
                        row.local_time,
                        row.interval_minutes,
                    )?,
                })
            })
            .collect()
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
        let rows = sqlx::query_scalar::<_, Uuid>(
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
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(rows.len() as u64)
    }
}

#[cfg(test)]
#[path = "timer_tests.rs"]
mod tests;
