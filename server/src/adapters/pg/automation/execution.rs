// Idempotent webhook deliveries and durable automation runs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::automation_config::{
    AutomationRun, AutomationRunStatus, WebhookDelivery, WebhookDeliveryStatus,
};
use crate::domain::error::DomainError;
use crate::ports::{
    AutomationRunRepo, AutomationRunReservation, WebhookDeliveryClaim, WebhookDeliveryRepo,
};

pub struct PgWebhookDeliveryRepo {
    pool: PgPool,
}

impl PgWebhookDeliveryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct WebhookDeliveryRow {
    id: Uuid,
    connection_id: Uuid,
    provider_delivery_id: String,
    provider_event: String,
    status: String,
    error_code: Option<String>,
    received_at: DateTime<Utc>,
}

impl TryFrom<WebhookDeliveryRow> for WebhookDelivery {
    type Error = DomainError;

    fn try_from(row: WebhookDeliveryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            connection_id: row.connection_id,
            provider_delivery_id: row.provider_delivery_id,
            provider_event: row.provider_event,
            status: WebhookDeliveryStatus::from_stored(&row.status)?,
            error_code: row.error_code,
            received_at: row.received_at,
        })
    }
}

#[async_trait]
impl WebhookDeliveryRepo for PgWebhookDeliveryRepo {
    async fn claim_delivery(
        &self,
        delivery: &WebhookDelivery,
    ) -> Result<Option<WebhookDeliveryClaim>, DomainError> {
        if delivery.status != WebhookDeliveryStatus::Received || delivery.error_code.is_some() {
            return Err(DomainError::InvalidWebhookDelivery);
        }
        let token = Uuid::new_v4();
        let delivery_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO webhook_deliveries (
                id, connection_id, provider_delivery_id, provider_event,
                status, error_code, received_at, claim_token, claim_expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now() + interval '15 minutes')
            ON CONFLICT (connection_id, provider_delivery_id) DO UPDATE
            SET claim_token = EXCLUDED.claim_token,
                claim_expires_at = EXCLUDED.claim_expires_at
            WHERE webhook_deliveries.status = 'received'
              AND webhook_deliveries.provider_event = EXCLUDED.provider_event
              AND webhook_deliveries.claim_expires_at <= now()
            RETURNING webhook_deliveries.id
            "#,
        )
        .bind(delivery.id)
        .bind(delivery.connection_id)
        .bind(&delivery.provider_delivery_id)
        .bind(&delivery.provider_event)
        .bind(delivery.status.to_string())
        .bind(&delivery.error_code)
        .bind(delivery.received_at)
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(delivery_id.map(|delivery_id| WebhookDeliveryClaim { delivery_id, token }))
    }

    async fn complete_claimed_delivery(
        &self,
        delivery: &WebhookDelivery,
        claim: WebhookDeliveryClaim,
    ) -> Result<bool, DomainError> {
        if delivery.id != claim.delivery_id || delivery.status == WebhookDeliveryStatus::Received {
            return Err(DomainError::InvalidWebhookDelivery);
        }
        let result = sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $2, error_code = $3,
                claim_token = NULL, claim_expires_at = NULL
            WHERE id = $1 AND status = 'received' AND claim_token = $4
            "#,
        )
        .bind(delivery.id)
        .bind(delivery.status.to_string())
        .bind(&delivery.error_code)
        .bind(claim.token)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $2, error_code = $3
            WHERE id = $1 AND status = 'received'
            "#,
        )
        .bind(delivery.id)
        .bind(delivery.status.to_string())
        .bind(&delivery.error_code)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn list_deliveries_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let rows = sqlx::query_as::<_, WebhookDeliveryRow>(
            r#"
            SELECT d.id, d.connection_id, d.provider_delivery_id,
                   d.provider_event, d.status, d.error_code, d.received_at
            FROM webhook_deliveries d
            JOIN service_connections c ON c.id = d.connection_id
            WHERE c.team_id = $1
            ORDER BY d.received_at DESC, d.id DESC
            LIMIT $2
            "#,
        )
        .bind(team_id)
        .bind(i64::from(limit.clamp(1, 200)))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

pub struct PgAutomationRunRepo {
    pool: PgPool,
}

impl PgAutomationRunRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct AutomationRunRow {
    id: Uuid,
    delivery_id: Uuid,
    rule_id: Option<Uuid>,
    status: String,
    incident_id: Option<Uuid>,
    error_code: Option<String>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<AutomationRunRow> for AutomationRun {
    type Error = DomainError;

    fn try_from(row: AutomationRunRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            delivery_id: row.delivery_id,
            rule_id: row.rule_id,
            status: AutomationRunStatus::from_stored(&row.status)?,
            incident_id: row.incident_id,
            error_code: row.error_code,
            started_at: row.started_at,
            finished_at: row.finished_at,
        })
    }
}

#[async_trait]
impl AutomationRunRepo for PgAutomationRunRepo {
    async fn reserve_run(
        &self,
        run: &AutomationRun,
        claim: WebhookDeliveryClaim,
    ) -> Result<AutomationRunReservation, DomainError> {
        if run.status != AutomationRunStatus::Running
            || run.incident_id.is_some()
            || run.error_code.is_some()
            || run.finished_at.is_some()
        {
            return Err(DomainError::InvalidAutomationRun);
        }
        let rule_id = run.rule_id.ok_or(DomainError::InvalidAutomationRule)?;
        // The INSERT ... SELECT binds a run to a rule triggered by the exact
        // connection that received the delivery. It prevents hand-crafted
        // cross-Team runs even before the application use case exists.
        let result = sqlx::query(
            r#"
            INSERT INTO automation_runs (
                id, delivery_id, rule_id, status, incident_id, error_code,
                started_at, finished_at
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8
            FROM webhook_deliveries d
            JOIN automation_rules r
              ON r.id = $3
             AND r.trigger_connection_id = d.connection_id
            WHERE d.id = $2
              AND d.claim_token = $9 AND d.claim_expires_at > now()
            ON CONFLICT (delivery_id, rule_id) DO NOTHING
            "#,
        )
        .bind(run.id)
        .bind(run.delivery_id)
        .bind(rule_id)
        .bind(run.status.to_string())
        .bind(run.incident_id)
        .bind(&run.error_code)
        .bind(run.started_at)
        .bind(run.finished_at)
        .bind(claim.token)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() == 1 {
            return Ok(AutomationRunReservation::New(run.clone()));
        }
        let existing = sqlx::query_as::<_, AutomationRunRow>(
            r#"
            SELECT id, delivery_id, rule_id, status, incident_id, error_code,
                   started_at, finished_at
            FROM automation_runs
            WHERE delivery_id = $1 AND rule_id = $2
            "#,
        )
        .bind(run.delivery_id)
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?
        .ok_or(DomainError::Storage)?;
        Ok(AutomationRunReservation::Existing(existing.try_into()?))
    }

    async fn update_run(&self, run: &AutomationRun) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE automation_runs
            SET status = $2,
                incident_id = $3,
                error_code = $4,
                finished_at = $5
            WHERE id = $1 AND status = 'running'
            "#,
        )
        .bind(run.id)
        .bind(run.status.to_string())
        .bind(run.incident_id)
        .bind(&run.error_code)
        .bind(run.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn interrupt_running_for_delivery(
        &self,
        claim: WebhookDeliveryClaim,
    ) -> Result<u64, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE automation_runs AS run
            SET status = 'failed', error_code = 'interrupted', finished_at = now()
            FROM webhook_deliveries AS delivery
            WHERE run.delivery_id = delivery.id
              AND delivery.id = $1 AND delivery.claim_token = $2
              AND delivery.claim_expires_at > now()
              AND run.status = 'running'
            "#,
        )
        .bind(claim.delivery_id)
        .bind(claim.token)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected())
    }

    async fn list_runs_for_team(
        &self,
        team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<AutomationRun>, DomainError> {
        let rows = sqlx::query_as::<_, AutomationRunRow>(
            r#"
            SELECT ar.id, ar.delivery_id, ar.rule_id, ar.status,
                   ar.incident_id, ar.error_code, ar.started_at, ar.finished_at
            FROM automation_runs ar
            JOIN webhook_deliveries d ON d.id = ar.delivery_id
            JOIN service_connections c ON c.id = d.connection_id
            WHERE c.team_id = $1
            ORDER BY ar.started_at DESC, ar.id DESC
            LIMIT $2
            "#,
        )
        .bind(team_id)
        .bind(i64::from(limit.clamp(1, 200)))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
