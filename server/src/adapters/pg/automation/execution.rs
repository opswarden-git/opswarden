// Idempotent webhook deliveries and durable automation runs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::automation_config::{
    AutomationRun, AutomationRunStatus, WebhookDelivery, WebhookDeliveryStatus,
};
use crate::domain::error::DomainError;
use crate::ports::{AutomationRunRepo, WebhookDeliveryClaim, WebhookDeliveryRepo};

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
    async fn insert_run(&self, run: &AutomationRun) -> Result<(), DomainError> {
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
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::Storage);
        }
        Ok(())
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
mod tests {
    use serde_json::json;

    use super::super::rule::PgAutomationRuleRepo;
    use super::super::service_connection::PgServiceConnectionRepo;
    use super::super::test_support::seed_team;
    use super::*;
    use crate::domain::automation_config::{AutomationRule, ServiceConnection};
    use crate::ports::{AutomationRuleRepo, ServiceConnectionRepo};

    async fn setup_rule(pool: &PgPool, suffix: &str) -> (Uuid, ServiceConnection, AutomationRule) {
        let (team_id, user_id) = seed_team(pool, suffix).await;
        let connections = PgServiceConnectionRepo::new(pool.clone());
        let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
        connections.insert_connection(&connection).await.unwrap();
        let mut rule = AutomationRule::new(
            team_id,
            format!("Rule {suffix}"),
            connection.id,
            "ci_failed",
            json!({}),
            "create_incident",
            None,
            json!({"severity": "high"}),
            user_id,
        )
        .unwrap();
        rule.set_enabled(true);
        PgAutomationRuleRepo::new(pool.clone())
            .insert_rule(&rule)
            .await
            .unwrap();
        (team_id, connection, rule)
    }

    #[sqlx::test]
    async fn provider_delivery_is_reserved_once_per_connection(pool: PgPool) {
        let (team_a, connection_a, _) = setup_rule(&pool, "delivery-a").await;
        let (team_b, connection_b, _) = setup_rule(&pool, "delivery-b").await;
        let repo = PgWebhookDeliveryRepo::new(pool);
        let delivery_a =
            WebhookDelivery::new(connection_a.id, "github-delivery-42", "workflow_run").unwrap();
        let delivery_b =
            WebhookDelivery::new(connection_b.id, "github-delivery-42", "workflow_run").unwrap();

        let claim_a = repo.claim_delivery(&delivery_a).await.unwrap().unwrap();
        assert!(repo.claim_delivery(&delivery_a).await.unwrap().is_none());
        assert!(repo.claim_delivery(&delivery_b).await.unwrap().is_some());

        let mut processed_a = delivery_a.clone();
        processed_a.mark_processed().unwrap();
        assert!(repo
            .complete_claimed_delivery(&processed_a, claim_a)
            .await
            .unwrap());
        assert!(!repo
            .complete_claimed_delivery(&processed_a, claim_a)
            .await
            .unwrap());

        let mut already_terminal =
            WebhookDelivery::new(connection_a.id, "already-terminal", "workflow_run").unwrap();
        already_terminal.mark_ignored().unwrap();
        assert_eq!(
            repo.claim_delivery(&already_terminal).await.unwrap_err(),
            DomainError::InvalidWebhookDelivery
        );
        assert_eq!(
            repo.list_deliveries_for_team(team_a, 20).await.unwrap(),
            vec![processed_a]
        );
        assert_eq!(
            repo.list_deliveries_for_team(team_b, 20).await.unwrap(),
            vec![delivery_b]
        );
    }

    #[sqlx::test]
    async fn expired_delivery_claim_is_recoverable_and_fences_stale_worker(pool: PgPool) {
        let (_, connection, _) = setup_rule(&pool, "delivery-reclaim").await;
        let repo = PgWebhookDeliveryRepo::new(pool.clone());
        let original =
            WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
        let stale_claim = repo.claim_delivery(&original).await.unwrap().unwrap();

        sqlx::query(
            "UPDATE webhook_deliveries SET claim_expires_at = now() - interval '1 second' WHERE id = $1",
        )
        .bind(stale_claim.delivery_id)
        .execute(&pool)
        .await
        .unwrap();

        let mut retry =
            WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
        let active_claim = repo.claim_delivery(&retry).await.unwrap().unwrap();
        assert_eq!(active_claim.delivery_id, stale_claim.delivery_id);
        assert_ne!(active_claim.token, stale_claim.token);
        retry.id = active_claim.delivery_id;
        retry.mark_processed().unwrap();

        assert!(!repo
            .complete_claimed_delivery(&retry, stale_claim)
            .await
            .unwrap());
        assert!(repo
            .complete_claimed_delivery(&retry, active_claim)
            .await
            .unwrap());
        let terminal_retry =
            WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
        assert!(repo
            .claim_delivery(&terminal_retry)
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn concurrent_delivery_claim_has_a_single_owner(pool: PgPool) {
        let (_, connection, _) = setup_rule(&pool, "delivery-concurrent").await;
        let delivery =
            WebhookDelivery::new(connection.id, "concurrent-delivery", "workflow_run").unwrap();
        let other = delivery.clone();
        let first_repo = PgWebhookDeliveryRepo::new(pool.clone());
        let second_repo = PgWebhookDeliveryRepo::new(pool);

        let (first, second) = tokio::join!(
            first_repo.claim_delivery(&delivery),
            second_repo.claim_delivery(&other)
        );
        assert_eq!(
            usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()),
            1
        );
    }

    #[sqlx::test]
    async fn runs_persist_terminal_state_and_remain_team_scoped(pool: PgPool) {
        let (team_a, connection_a, rule_a) = setup_rule(&pool, "run-a").await;
        let (team_b, _, _) = setup_rule(&pool, "run-b").await;
        let deliveries = PgWebhookDeliveryRepo::new(pool.clone());
        let delivery =
            WebhookDelivery::new(connection_a.id, "run-delivery", "workflow_run").unwrap();
        deliveries.claim_delivery(&delivery).await.unwrap().unwrap();

        let runs = PgAutomationRunRepo::new(pool);
        let mut run = AutomationRun::new(delivery.id, rule_a.id);
        runs.insert_run(&run).await.unwrap();
        assert_eq!(runs.list_runs_for_team(team_b, 20).await.unwrap(), vec![]);

        run.mark_succeeded(None).unwrap();
        assert!(runs.update_run(&run).await.unwrap());
        assert!(!runs.update_run(&run).await.unwrap());
        assert_eq!(
            runs.list_runs_for_team(team_a, 20).await.unwrap(),
            vec![run]
        );
    }

    #[sqlx::test]
    async fn run_cannot_pair_delivery_with_rule_from_another_connection(pool: PgPool) {
        let (_, connection_a, _) = setup_rule(&pool, "run-cross-a").await;
        let (_, _, rule_b) = setup_rule(&pool, "run-cross-b").await;
        let deliveries = PgWebhookDeliveryRepo::new(pool.clone());
        let delivery =
            WebhookDelivery::new(connection_a.id, "cross-delivery", "workflow_run").unwrap();
        deliveries.claim_delivery(&delivery).await.unwrap().unwrap();

        let runs = PgAutomationRunRepo::new(pool);
        let run = AutomationRun::new(delivery.id, rule_b.id);
        assert_eq!(
            runs.insert_run(&run).await.unwrap_err(),
            DomainError::Storage
        );
    }
}
