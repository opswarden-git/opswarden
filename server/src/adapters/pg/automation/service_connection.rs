// Team-scoped connection metadata and encrypted credentials.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::adapters::crypto::aes;
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::ports::{ConnectionCredentialVault, ServiceConnectionRepo};

pub struct PgServiceConnectionRepo {
    pool: PgPool,
}

impl PgServiceConnectionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn connection_from_row(row: &sqlx::postgres::PgRow) -> Result<ServiceConnection, DomainError> {
    Ok(ServiceConnection {
        id: row.try_get("id").map_err(|_| DomainError::Storage)?,
        team_id: row.try_get("team_id").map_err(|_| DomainError::Storage)?,
        service: row.try_get("service").map_err(|_| DomainError::Storage)?,
        created_by: row
            .try_get("created_by")
            .map_err(|_| DomainError::Storage)?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| DomainError::Storage)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|_| DomainError::Storage)?,
        verified_at: row
            .try_get("verified_at")
            .map_err(|_| DomainError::Storage)?,
        last_delivery_at: row
            .try_get("last_delivery_at")
            .map_err(|_| DomainError::Storage)?,
        last_error_code: row
            .try_get("last_error_code")
            .map_err(|_| DomainError::Storage)?,
    })
}

#[async_trait]
impl ServiceConnectionRepo for PgServiceConnectionRepo {
    async fn insert_connection(&self, connection: &ServiceConnection) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO service_connections
                (id, team_id, service, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(connection.id)
        .bind(connection.team_id)
        .bind(&connection.service)
        .bind(connection.created_by)
        .bind(connection.created_at)
        .bind(connection.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn find_connection_by_id(
        &self,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, team_id, service, created_by, created_at, updated_at,
                   verified_at, last_delivery_at, last_error_code
            FROM service_connections
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        row.as_ref().map(connection_from_row).transpose()
    }

    async fn find_connection_for_team(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, team_id, service, created_by, created_at, updated_at,
                   verified_at, last_delivery_at, last_error_code
            FROM service_connections
            WHERE team_id = $1 AND id = $2
            "#,
        )
        .bind(team_id)
        .bind(connection_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        row.as_ref().map(connection_from_row).transpose()
    }

    async fn find_connection_by_service(
        &self,
        team_id: Uuid,
        service: &str,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT id, team_id, service, created_by, created_at, updated_at,
                   verified_at, last_delivery_at, last_error_code
            FROM service_connections
            WHERE team_id = $1 AND service = $2
            "#,
        )
        .bind(team_id)
        .bind(service)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        row.as_ref().map(connection_from_row).transpose()
    }

    async fn list_connections_for_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<ServiceConnection>, DomainError> {
        sqlx::query(
            r#"
            SELECT id, team_id, service, created_by, created_at, updated_at,
                   verified_at, last_delivery_at, last_error_code
            FROM service_connections
            WHERE team_id = $1
            ORDER BY service, id
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?
        .iter()
        .map(connection_from_row)
        .collect()
    }

    async fn record_delivery_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE service_connections
            SET verified_at = coalesce(verified_at, now()),
                last_delivery_at = now(),
                last_error_code = $2,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .bind(error_code)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        Ok(())
    }

    async fn record_reaction_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE service_connections
            SET verified_at = CASE
                    WHEN $2::text IS NULL THEN coalesce(verified_at, now())
                    ELSE verified_at
                END,
                last_error_code = $2,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .bind(error_code)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        Ok(())
    }

    async fn reset_connection_health(&self, connection_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE service_connections
            SET verified_at = NULL,
                last_error_code = NULL,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(connection_id)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        Ok(())
    }

    async fn delete_connection(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM service_connections WHERE team_id = $1 AND id = $2")
            .bind(team_id)
            .bind(connection_id)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(result.rows_affected() == 1)
    }
}

pub struct PgConnectionCredentialVault {
    pool: PgPool,
    key: [u8; aes::KEY_LEN],
}

impl PgConnectionCredentialVault {
    pub fn new(pool: PgPool, key: [u8; aes::KEY_LEN]) -> Self {
        Self { pool, key }
    }
}

#[async_trait]
impl ConnectionCredentialVault for PgConnectionCredentialVault {
    async fn store_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
        secret: &str,
    ) -> Result<(), DomainError> {
        if secret.trim().is_empty() {
            return Err(DomainError::InvalidServiceSecret);
        }
        let (nonce, ciphertext) = aes::encrypt(&self.key, secret.as_bytes())?;
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        sqlx::query(
            r#"
            INSERT INTO service_connection_secrets
                (connection_id, kind, nonce, ciphertext, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (connection_id, kind) DO UPDATE
            SET nonce = excluded.nonce,
                ciphertext = excluded.ciphertext,
                updated_at = now()
            "#,
        )
        .bind(connection_id)
        .bind(kind.to_string())
        .bind(nonce)
        .bind(ciphertext)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        let touched =
            sqlx::query("UPDATE service_connections SET updated_at = now() WHERE id = $1")
                .bind(connection_id)
                .execute(&mut *tx)
                .await
                .map_err(|_| DomainError::Storage)?;
        if touched.rows_affected() != 1 {
            return Err(DomainError::Storage);
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn reveal_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<Option<String>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT nonce, ciphertext
            FROM service_connection_secrets
            WHERE connection_id = $1 AND kind = $2
            "#,
        )
        .bind(connection_id)
        .bind(kind.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let Some(row) = row else {
            return Ok(None);
        };
        let nonce: Vec<u8> = row.try_get("nonce").map_err(|_| DomainError::Storage)?;
        let ciphertext: Vec<u8> = row
            .try_get("ciphertext")
            .map_err(|_| DomainError::Storage)?;
        let plaintext = aes::decrypt(&self.key, &nonce, &ciphertext)?;
        String::from_utf8(plaintext)
            .map(Some)
            .map_err(|_| DomainError::Crypto)
    }

    async fn delete_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        sqlx::query(
            "DELETE FROM service_connection_secrets WHERE connection_id = $1 AND kind = $2",
        )
        .bind(connection_id)
        .bind(kind.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        let touched =
            sqlx::query("UPDATE service_connections SET updated_at = now() WHERE id = $1")
                .bind(connection_id)
                .execute(&mut *tx)
                .await
                .map_err(|_| DomainError::Storage)?;
        if touched.rows_affected() != 1 {
            return Err(DomainError::Storage);
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn configured_credential_kinds(
        &self,
        connection_id: Uuid,
    ) -> Result<Vec<CredentialKind>, DomainError> {
        let rows = sqlx::query(
            "SELECT kind FROM service_connection_secrets WHERE connection_id = $1 ORDER BY kind",
        )
        .bind(connection_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        rows.iter()
            .map(|row| {
                let value: String = row.try_get("kind").map_err(|_| DomainError::Storage)?;
                CredentialKind::from_stored(&value)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::seed_team;
    use super::*;

    const KEY: [u8; aes::KEY_LEN] = [73; aes::KEY_LEN];

    #[sqlx::test]
    async fn the_same_provider_is_isolated_between_teams(pool: PgPool) {
        let (team_a, user_a) = seed_team(&pool, "connections-a").await;
        let (team_b, user_b) = seed_team(&pool, "connections-b").await;
        let repo = PgServiceConnectionRepo::new(pool.clone());
        let github_a = ServiceConnection::new(team_a, "github", user_a).unwrap();
        let github_b = ServiceConnection::new(team_b, "github", user_b).unwrap();
        repo.insert_connection(&github_a).await.unwrap();
        repo.insert_connection(&github_b).await.unwrap();

        assert_eq!(
            repo.list_connections_for_team(team_a).await.unwrap(),
            vec![github_a.clone()]
        );
        assert_eq!(
            repo.list_connections_for_team(team_b).await.unwrap(),
            vec![github_b.clone()]
        );
        assert!(repo
            .find_connection_for_team(team_a, github_b.id)
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn credentials_are_encrypted_and_separated_by_connection_and_kind(pool: PgPool) {
        let (team_a, user_a) = seed_team(&pool, "vault-a").await;
        let (team_b, user_b) = seed_team(&pool, "vault-b").await;
        let repo = PgServiceConnectionRepo::new(pool.clone());
        let github_a = ServiceConnection::new(team_a, "github", user_a).unwrap();
        let github_b = ServiceConnection::new(team_b, "github", user_b).unwrap();
        repo.insert_connection(&github_a).await.unwrap();
        repo.insert_connection(&github_b).await.unwrap();

        let vault = PgConnectionCredentialVault::new(pool.clone(), KEY);
        vault
            .store_credential(
                github_a.id,
                CredentialKind::WebhookSigningSecret,
                "team-a-signing-secret",
            )
            .await
            .unwrap();
        vault
            .store_credential(
                github_a.id,
                CredentialKind::PersonalToken,
                "github_pat_team_a",
            )
            .await
            .unwrap();
        vault
            .store_credential(
                github_b.id,
                CredentialKind::WebhookSigningSecret,
                "team-b-signing-secret",
            )
            .await
            .unwrap();

        assert_eq!(
            vault
                .reveal_credential(github_a.id, CredentialKind::WebhookSigningSecret)
                .await
                .unwrap()
                .as_deref(),
            Some("team-a-signing-secret")
        );
        assert_eq!(
            vault
                .reveal_credential(github_b.id, CredentialKind::WebhookSigningSecret)
                .await
                .unwrap()
                .as_deref(),
            Some("team-b-signing-secret")
        );

        let row = sqlx::query(
            "SELECT ciphertext FROM service_connection_secrets WHERE connection_id = $1 AND kind = $2",
        )
        .bind(github_a.id)
        .bind(CredentialKind::PersonalToken.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        let ciphertext: Vec<u8> = row.try_get("ciphertext").unwrap();
        assert_ne!(ciphertext, b"github_pat_team_a");

        assert_eq!(
            vault
                .configured_credential_kinds(github_a.id)
                .await
                .unwrap(),
            vec![
                CredentialKind::PersonalToken,
                CredentialKind::WebhookSigningSecret
            ]
        );
        let touched_connection = repo
            .find_connection_for_team(team_a, github_a.id)
            .await
            .unwrap()
            .unwrap();
        assert!(touched_connection.updated_at >= github_a.updated_at);
    }

    #[sqlx::test]
    async fn deleting_connection_cascades_credentials_but_is_team_scoped(pool: PgPool) {
        let (team_a, user_a) = seed_team(&pool, "delete-a").await;
        let (team_b, user_b) = seed_team(&pool, "delete-b").await;
        let repo = PgServiceConnectionRepo::new(pool.clone());
        let connection = ServiceConnection::new(team_a, "github", user_a).unwrap();
        let other = ServiceConnection::new(team_b, "github", user_b).unwrap();
        repo.insert_connection(&connection).await.unwrap();
        repo.insert_connection(&other).await.unwrap();
        let vault = PgConnectionCredentialVault::new(pool, KEY);
        vault
            .store_credential(
                connection.id,
                CredentialKind::WebhookSigningSecret,
                "secret",
            )
            .await
            .unwrap();

        assert!(!repo.delete_connection(team_b, connection.id).await.unwrap());
        assert!(vault
            .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
            .await
            .unwrap()
            .is_some());
        assert!(repo.delete_connection(team_a, connection.id).await.unwrap());
        assert!(vault
            .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn signed_delivery_health_is_persisted_without_resetting_first_verification(
        pool: PgPool,
    ) {
        let (team_id, user_id) = seed_team(&pool, "delivery-health").await;
        let repo = PgServiceConnectionRepo::new(pool);
        let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
        repo.insert_connection(&connection).await.unwrap();

        repo.record_delivery_result(connection.id, None)
            .await
            .unwrap();
        let verified = repo
            .find_connection_for_team(team_id, connection.id)
            .await
            .unwrap()
            .unwrap();
        assert!(verified.verified_at.is_some());
        assert!(verified.last_delivery_at.is_some());
        assert_eq!(verified.last_error_code, None);

        repo.record_delivery_result(connection.id, Some("invalid_automation_rule"))
            .await
            .unwrap();
        let failed = repo
            .find_connection_for_team(team_id, connection.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.verified_at, verified.verified_at);
        assert!(failed.last_delivery_at >= verified.last_delivery_at);
        assert_eq!(
            failed.last_error_code.as_deref(),
            Some("invalid_automation_rule")
        );
    }

    #[sqlx::test]
    async fn outbound_health_does_not_claim_an_inbound_delivery_and_resets_on_replace(
        pool: PgPool,
    ) {
        let (team_id, user_id) = seed_team(&pool, "reaction-health").await;
        let repo = PgServiceConnectionRepo::new(pool);
        let connection = ServiceConnection::new(team_id, "http", user_id).unwrap();
        repo.insert_connection(&connection).await.unwrap();

        repo.record_reaction_result(connection.id, None)
            .await
            .unwrap();
        let verified = repo
            .find_connection_for_team(team_id, connection.id)
            .await
            .unwrap()
            .unwrap();
        assert!(verified.verified_at.is_some());
        assert!(verified.last_delivery_at.is_none());

        repo.record_reaction_result(connection.id, Some("reaction_http_5xx"))
            .await
            .unwrap();
        let failed = repo
            .find_connection_for_team(team_id, connection.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.last_error_code.as_deref(), Some("reaction_http_5xx"));

        repo.reset_connection_health(connection.id).await.unwrap();
        let reset = repo
            .find_connection_for_team(team_id, connection.id)
            .await
            .unwrap()
            .unwrap();
        assert!(reset.verified_at.is_none());
        assert!(reset.last_error_code.is_none());
        assert!(reset.last_delivery_at.is_none());
    }
}
