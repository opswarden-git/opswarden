// --- server/src/adapters/pg/vault.rs ---
//
// SecretVault backed by Postgres + AES-256-GCM. Only ciphertext and a per-row
// nonce ever touch the database, so a raw `SELECT * FROM external_secrets`
// reveals nothing usable. Queries use the compile-checked `sqlx::query!` macro,
// like the rest of the pg adapters (validated against the schema in `.sqlx`).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::crypto::aes;
use crate::domain::error::DomainError;
use crate::ports::SecretVault;

pub struct PgAesVault {
    pool: PgPool,
    key: [u8; aes::KEY_LEN],
}

impl PgAesVault {
    pub fn new(pool: PgPool, key: [u8; aes::KEY_LEN]) -> Self {
        Self { pool, key }
    }
}

#[async_trait]
impl SecretVault for PgAesVault {
    async fn store(&self, service: &str, secret: &str) -> Result<(), DomainError> {
        let (nonce, ciphertext) = aes::encrypt(&self.key, secret.as_bytes())?;

        sqlx::query!(
            r#"
            INSERT INTO external_secrets (service, nonce, ciphertext, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (service)
            DO UPDATE SET nonce = EXCLUDED.nonce,
                          ciphertext = EXCLUDED.ciphertext,
                          updated_at = now()
            "#,
            service,
            nonce,
            ciphertext
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn reveal(&self, service: &str) -> Result<Option<String>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT nonce, ciphertext
            FROM external_secrets
            WHERE service = $1
            "#,
            service
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let Some(row) = record else {
            return Ok(None);
        };

        let plaintext = aes::decrypt(&self.key, &row.nonce, &row.ciphertext)?;
        let secret = String::from_utf8(plaintext).map_err(|_| DomainError::Crypto)?;
        Ok(Some(secret))
    }

    async fn delete(&self, service: &str) -> Result<(), DomainError> {
        sqlx::query!("DELETE FROM external_secrets WHERE service = $1", service)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(())
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::Row;

    const KEY: [u8; aes::KEY_LEN] = [42u8; aes::KEY_LEN];

    async fn pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string()
        });
        PgPoolOptions::new().connect(&database_url).await.unwrap()
    }

    #[tokio::test]
    async fn it_stores_and_reveals_a_secret_in_postgres() {
        let vault = PgAesVault::new(pool().await, KEY);
        let service = format!("svc_{}", uuid::Uuid::new_v4());

        assert!(vault.reveal(&service).await.unwrap().is_none());

        vault.store(&service, "ghp_top_secret").await.unwrap();
        // Upsert: a second store overwrites rather than conflicting.
        vault.store(&service, "ghp_rotated").await.unwrap();

        assert_eq!(
            vault.reveal(&service).await.unwrap().as_deref(),
            Some("ghp_rotated")
        );
    }

    #[tokio::test]
    async fn raw_row_holds_ciphertext_not_the_plaintext() {
        let p = pool().await;
        let vault = PgAesVault::new(p.clone(), KEY);
        let service = format!("svc_{}", uuid::Uuid::new_v4());
        vault.store(&service, "ghp_top_secret").await.unwrap();

        // What a raw operator SELECT would see: encrypted bytes, never the secret.
        let row = sqlx::query("SELECT ciphertext FROM external_secrets WHERE service = $1")
            .bind(&service)
            .fetch_one(&p)
            .await
            .unwrap();
        let ciphertext: Vec<u8> = row.get("ciphertext");
        assert_ne!(ciphertext.as_slice(), b"ghp_top_secret");
    }

    #[tokio::test]
    async fn delete_removes_the_secret_idempotently() {
        let vault = PgAesVault::new(pool().await, KEY);
        let service = format!("svc_{}", uuid::Uuid::new_v4());
        vault.store(&service, "to-be-deleted").await.unwrap();
        assert!(vault.reveal(&service).await.unwrap().is_some());

        vault.delete(&service).await.unwrap();
        assert!(vault.reveal(&service).await.unwrap().is_none());
        // Deleting a missing service is not an error.
        vault.delete(&service).await.unwrap();
    }
}
