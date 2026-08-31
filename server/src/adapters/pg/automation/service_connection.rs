// Team-scoped connection metadata and encrypted credentials.

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::adapters::crypto::aes;
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::ports::{
    ConnectionCredentialVault, ConnectionHealthMutation, CredentialMutation, ServiceConnectionRepo,
};

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
    async fn configure_connection(
        &self,
        connection: &ServiceConnection,
        credentials: &[CredentialMutation],
        health: ConnectionHealthMutation,
    ) -> Result<ServiceConnection, DomainError> {
        let mut encrypted = Vec::with_capacity(credentials.len());
        for mutation in credentials {
            let value = match mutation.secret.as_deref() {
                Some(secret) => {
                    if secret.trim().is_empty() {
                        return Err(DomainError::InvalidServiceSecret);
                    }
                    let aad = aes::canonical_aad(
                        &connection.id,
                        &mutation.kind.to_string(),
                        aes::DEFAULT_KEY_VERSION,
                    );
                    Some(aes::encrypt(&self.key, secret.as_bytes(), &aad)?)
                }
                None => None,
            };
            encrypted.push((mutation.kind, value));
        }

        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let row = sqlx::query(
            r#"
            INSERT INTO service_connections
                (id, team_id, service, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (team_id, service) DO UPDATE
            SET service = excluded.service
            RETURNING id, team_id, service, created_by, created_at, updated_at,
                      verified_at, last_delivery_at, last_error_code
            "#,
        )
        .bind(connection.id)
        .bind(connection.team_id)
        .bind(&connection.service)
        .bind(connection.created_by)
        .bind(connection.created_at)
        .bind(connection.updated_at)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        let stored = connection_from_row(&row)?;

        for (kind, value) in encrypted {
            match value {
                Some((nonce, ciphertext)) => {
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
                    .bind(stored.id)
                    .bind(kind.to_string())
                    .bind(nonce)
                    .bind(ciphertext)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| DomainError::Storage)?;
                }
                None => {
                    sqlx::query(
                        "DELETE FROM service_connection_secrets WHERE connection_id = $1 AND kind = $2",
                    )
                    .bind(stored.id)
                    .bind(kind.to_string())
                    .execute(&mut *transaction)
                    .await
                    .map_err(|_| DomainError::Storage)?;
                }
            }
        }

        let health = match health {
            ConnectionHealthMutation::Preserve => "preserve",
            ConnectionHealthMutation::Reset => "reset",
            ConnectionHealthMutation::Verified => "verified",
        };
        let row = sqlx::query(
            r#"
            UPDATE service_connections
            SET verified_at = CASE
                    WHEN $2 = 'reset' THEN NULL
                    WHEN $2 = 'verified' THEN coalesce(verified_at, now())
                    ELSE verified_at
                END,
                last_error_code = CASE
                    WHEN $2 IN ('reset', 'verified') THEN NULL
                    ELSE last_error_code
                END,
                updated_at = now()
            WHERE id = $1
            RETURNING id, team_id, service, created_by, created_at, updated_at,
                      verified_at, last_delivery_at, last_error_code
            "#,
        )
        .bind(stored.id)
        .bind(health)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        let stored = connection_from_row(&row)?;
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(stored)
    }

    async fn store_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
        secret: &str,
    ) -> Result<(), DomainError> {
        if secret.trim().is_empty() {
            return Err(DomainError::InvalidServiceSecret);
        }
        let aad = aes::canonical_aad(&connection_id, &kind.to_string(), aes::DEFAULT_KEY_VERSION);
        let (nonce, ciphertext) = aes::encrypt(&self.key, secret.as_bytes(), &aad)?;
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
        let aad = aes::canonical_aad(&connection_id, &kind.to_string(), aes::DEFAULT_KEY_VERSION);
        let plaintext = aes::decrypt(&self.key, &nonce, &ciphertext, &aad)?;
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
#[path = "service_connection_tests.rs"]
mod tests;
