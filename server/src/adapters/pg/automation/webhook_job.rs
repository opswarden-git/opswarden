use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::{ClaimedWebhookJob, WebhookJob, WebhookJobRepo};

pub struct PgWebhookJobRepo {
    pool: PgPool,
}

impl PgWebhookJobRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct WebhookJobRow {
    id: Uuid,
    connection_id: Uuid,
    expected_service: String,
    provider_delivery_id: String,
    provider_event: String,
    body: Vec<u8>,
    claim_token: Uuid,
}

impl From<WebhookJobRow> for ClaimedWebhookJob {
    fn from(row: WebhookJobRow) -> Self {
        Self {
            job: WebhookJob {
                id: row.id,
                connection_id: row.connection_id,
                expected_service: row.expected_service,
                provider_delivery_id: row.provider_delivery_id,
                provider_event: row.provider_event,
                body: row.body,
            },
            token: row.claim_token,
        }
    }
}

#[async_trait]
impl WebhookJobRepo for PgWebhookJobRepo {
    async fn enqueue(&self, job: &WebhookJob) -> Result<bool, DomainError> {
        Ok(self.enqueue_batch(std::slice::from_ref(job)).await?[0])
    }

    async fn enqueue_batch(&self, jobs: &[WebhookJob]) -> Result<Vec<bool>, DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let mut accepted = Vec::with_capacity(jobs.len());
        for job in jobs {
            let result = sqlx::query(
                r#"
                INSERT INTO webhook_jobs (
                    id, connection_id, expected_service, provider_delivery_id,
                    provider_event, body
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (connection_id, provider_delivery_id) DO NOTHING
                "#,
            )
            .bind(job.id)
            .bind(job.connection_id)
            .bind(&job.expected_service)
            .bind(&job.provider_delivery_id)
            .bind(&job.provider_event)
            .bind(&job.body)
            .execute(&mut *transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
            accepted.push(result.rows_affected() == 1);
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(accepted)
    }

    async fn claim(&self, limit: u32) -> Result<Vec<ClaimedWebhookJob>, DomainError> {
        let token = Uuid::new_v4();
        let rows = sqlx::query_as::<_, WebhookJobRow>(
            r#"
            WITH candidates AS (
                SELECT id
                FROM webhook_jobs
                WHERE available_at <= now()
                  AND (
                    status = 'queued'
                    OR (status = 'processing' AND claim_expires_at <= now())
                  )
                ORDER BY created_at, id
                FOR UPDATE SKIP LOCKED
                LIMIT $1
            )
            UPDATE webhook_jobs AS jobs
            SET status = 'processing', claim_token = $2,
                claim_expires_at = now() + interval '15 minutes',
                attempts = attempts + 1
            FROM candidates
            WHERE jobs.id = candidates.id
            RETURNING jobs.id, jobs.connection_id, jobs.expected_service,
                      jobs.provider_delivery_id, jobs.provider_event, jobs.body,
                      jobs.claim_token
            "#,
        )
        .bind(i64::from(limit.clamp(1, 100)))
        .bind(token)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn complete(&self, claim: &ClaimedWebhookJob) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE webhook_jobs
            SET status = 'completed', body = ''::bytea,
                claim_token = NULL, claim_expires_at = NULL,
                completed_at = now(), last_error_code = NULL
            WHERE id = $1 AND status = 'processing' AND claim_token = $2
            "#,
        )
        .bind(claim.job.id)
        .bind(claim.token)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }

    async fn retry(
        &self,
        claim: &ClaimedWebhookJob,
        error_code: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE webhook_jobs
            -- The delivery claim used by the execution pipeline lasts fifteen
            -- minutes. Retry after it expires so an abandoned attempt is
            -- reclaimed instead of mistaken for a completed duplicate.
            SET status = 'queued', available_at = now() + interval '16 minutes',
                claim_token = NULL, claim_expires_at = NULL,
                last_error_code = $3
            WHERE id = $1 AND status = 'processing' AND claim_token = $2
            "#,
        )
        .bind(claim.job.id)
        .bind(claim.token)
        .bind(error_code)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::automation::service_connection::PgServiceConnectionRepo;
    use crate::adapters::pg::automation::test_support::seed_team;
    use crate::domain::automation_config::ServiceConnection;
    use crate::ports::ServiceConnectionRepo;

    async fn job(pool: &PgPool, suffix: &str) -> WebhookJob {
        let (team_id, user_id) = seed_team(pool, suffix).await;
        let connection = ServiceConnection::new(team_id, "generic", user_id).unwrap();
        PgServiceConnectionRepo::new(pool.clone())
            .insert_connection(&connection)
            .await
            .unwrap();
        WebhookJob {
            id: Uuid::new_v4(),
            connection_id: connection.id,
            expected_service: "generic".to_string(),
            provider_delivery_id: format!("delivery-{suffix}"),
            provider_event: "generic_event".to_string(),
            body: br#"{"event_type":"alert"}"#.to_vec(),
        }
    }

    #[sqlx::test]
    async fn queue_deduplicates_and_fences_a_reclaimed_job(pool: PgPool) {
        let repo = PgWebhookJobRepo::new(pool.clone());
        let job = job(&pool, "queue-reclaim").await;
        let duplicate = WebhookJob {
            id: Uuid::new_v4(),
            ..job.clone()
        };
        assert_eq!(
            repo.enqueue_batch(&[job.clone(), duplicate]).await.unwrap(),
            vec![true, false]
        );
        assert!(!repo.enqueue(&job).await.unwrap());

        let stale = repo.claim(1).await.unwrap().pop().unwrap();
        assert!(repo.claim(1).await.unwrap().is_empty());
        sqlx::query(
            "UPDATE webhook_jobs SET claim_expires_at = now() - interval '1 second' WHERE id = $1",
        )
        .bind(job.id)
        .execute(&pool)
        .await
        .unwrap();
        let active = repo.claim(1).await.unwrap().pop().unwrap();
        assert_ne!(stale.token, active.token);
        assert!(!repo.complete(&stale).await.unwrap());
        assert!(repo.complete(&active).await.unwrap());
        assert!(repo.claim(1).await.unwrap().is_empty());

        let (status, body_len): (String, i32) =
            sqlx::query_as("SELECT status, octet_length(body) FROM webhook_jobs WHERE id = $1")
                .bind(job.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "completed");
        assert_eq!(body_len, 0);
    }

    #[sqlx::test]
    async fn invalid_batch_item_rolls_back_every_job(pool: PgPool) {
        let repo = PgWebhookJobRepo::new(pool.clone());
        let first = job(&pool, "queue-rollback").await;
        let invalid = WebhookJob {
            id: Uuid::new_v4(),
            connection_id: Uuid::new_v4(),
            provider_delivery_id: "invalid-foreign-connection".to_string(),
            ..first.clone()
        };

        assert_eq!(
            repo.enqueue_batch(&[first.clone(), invalid])
                .await
                .unwrap_err(),
            DomainError::Storage
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM webhook_jobs WHERE id = $1")
            .bind(first.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[sqlx::test]
    async fn retry_releases_the_claim_after_the_delivery_lease(pool: PgPool) {
        let repo = PgWebhookJobRepo::new(pool.clone());
        let job = job(&pool, "queue-retry").await;
        repo.enqueue(&job).await.unwrap();
        let claim = repo.claim(1).await.unwrap().pop().unwrap();

        assert!(repo.retry(&claim, "storage_error").await.unwrap());
        assert!(repo.claim(1).await.unwrap().is_empty());
        let (status, error): (String, Option<String>) =
            sqlx::query_as("SELECT status, last_error_code FROM webhook_jobs WHERE id = $1")
                .bind(job.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "queued");
        assert_eq!(error.as_deref(), Some("storage_error"));
    }
}
