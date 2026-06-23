use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::domain::error::DomainError;
use crate::ports::TokenRevocationRepo;

pub struct PgTokenRevocationRepo {
    pool: PgPool,
}

impl PgTokenRevocationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[async_trait]
impl TokenRevocationRepo for PgTokenRevocationRepo {
    async fn revoke(&self, token: &str, expires_at: DateTime<Utc>) -> Result<(), DomainError> {
        let token_hash = Self::hash_token(token);

        sqlx::query(
            r#"
            INSERT INTO revoked_tokens (token_hash, expires_at)
            VALUES ($1, $2)
            ON CONFLICT (token_hash) DO NOTHING
            "#,
        )
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn is_revoked(&self, token: &str) -> Result<bool, DomainError> {
        let token_hash = Self::hash_token(token);

        let record = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM revoked_tokens
                WHERE token_hash = $1
                  AND expires_at > now()
            )
            "#,
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[sqlx::test]
    async fn revoke_then_is_revoked(pool: PgPool) {
        let repo = PgTokenRevocationRepo::new(pool);
        let token = "jwt-token-abc";

        assert!(!repo.is_revoked(token).await.unwrap());
        repo.revoke(token, Utc::now() + Duration::hours(1))
            .await
            .unwrap();
        assert!(repo.is_revoked(token).await.unwrap());
    }

    #[sqlx::test]
    async fn an_expired_revocation_no_longer_counts(pool: PgPool) {
        let repo = PgTokenRevocationRepo::new(pool);
        let token = "jwt-token-expired";

        repo.revoke(token, Utc::now() - Duration::hours(1))
            .await
            .unwrap();
        assert!(!repo.is_revoked(token).await.unwrap());
    }

    #[sqlx::test]
    async fn revoking_twice_is_idempotent(pool: PgPool) {
        let repo = PgTokenRevocationRepo::new(pool);
        let token = "jwt-token-twice";
        let exp = Utc::now() + Duration::hours(1);

        repo.revoke(token, exp).await.unwrap();
        repo.revoke(token, exp).await.unwrap(); // ON CONFLICT DO NOTHING
        assert!(repo.is_revoked(token).await.unwrap());
    }
}
