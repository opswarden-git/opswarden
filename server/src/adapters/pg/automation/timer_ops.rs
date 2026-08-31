use super::*;

impl PgAutomationTimerRepo {
    pub(crate) async fn claim_due_impl(
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

    pub(crate) async fn start_execution_impl(
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

    pub(crate) async fn list_unstarted_claims_impl(
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
}
