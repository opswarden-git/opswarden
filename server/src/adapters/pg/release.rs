// --- server/src/adapters/pg/release.rs ---
//
// Postgres adapter for releases. The release row stores only the base lifecycle
// state; `blocked` is computed by callers from `count_active_linked_incidents`,
// which is the single SQL join over `incidents.status` that makes auto-unblock
// fall out of an incident resolving.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::automation_catalog::OPSWARDEN_SERVICE;
use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::release::{Release, ReleaseBaseState, ReleaseStep};
use crate::ports::ReleaseRepo;

use super::incident::{insert_event, insert_incident};

pub struct PgReleaseRepo {
    pool: PgPool,
}

impl PgReleaseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert_release(
        transaction: &mut Transaction<'_, Postgres>,
        release: &Release,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO releases (id, team_id, title, base_state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            release.id,
            release.team_id,
            release.title,
            release.base_state.as_str(),
            release.created_at,
            release.updated_at,
        )
        .execute(&mut **transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        for step in &release.steps {
            sqlx::query!(
                r#"
                INSERT INTO release_steps (release_id, position, name, validated_by, validated_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                release.id,
                step.position,
                step.name,
                step.validated_by,
                step.validated_at,
            )
            .execute(&mut **transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        Ok(())
    }

    async fn update_release_rows(
        transaction: &mut Transaction<'_, Postgres>,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let updated = sqlx::query(
            r#"
            UPDATE releases SET base_state = $2, updated_at = $3
            WHERE id = $1 AND updated_at = $4
            "#,
        )
        .bind(release.id)
        .bind(release.base_state.as_str())
        .bind(release.updated_at)
        .bind(expected_updated_at)
        .execute(&mut **transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if updated.rows_affected() != 1 {
            return Err(DomainError::ConcurrentModification);
        }

        for step in &release.steps {
            let updated = sqlx::query(
                r#"
                UPDATE release_steps SET validated_by = $3, validated_at = $4
                WHERE release_id = $1 AND position = $2
                "#,
            )
            .bind(release.id)
            .bind(step.position)
            .bind(step.validated_by)
            .bind(step.validated_at)
            .execute(&mut **transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
            if updated.rows_affected() != 1 {
                return Err(DomainError::ConcurrentModification);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ReleaseRepo for PgReleaseRepo {
    async fn save_release(&self, release: &Release) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::insert_release(&mut tx, release).await?;
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn create_release(
        &self,
        release: &Release,
        delivery_id: &str,
        event: &crate::domain::automation::ExternalEvent,
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::insert_release(&mut transaction, release).await?;
        let body = serde_json::to_vec(event).map_err(|_| DomainError::Storage)?;
        let result = sqlx::query(
            r#"
            INSERT INTO webhook_jobs (
                id, connection_id, expected_service, provider_delivery_id,
                provider_event, body
            )
            SELECT $1, connection.id, $2, $3, $4, $5
            FROM service_connections AS connection
            WHERE connection.team_id = $6 AND connection.service = $2
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(OPSWARDEN_SERVICE)
        .bind(delivery_id)
        .bind(&event.kind)
        .bind(body)
        .bind(release.team_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::Storage);
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn create_blocking_incident(
        &self,
        release_id: Uuid,
        expected_updated_at: DateTime<Utc>,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let locked = sqlx::query(
            r#"
            UPDATE releases
            SET updated_at = GREATEST(clock_timestamp(), updated_at + interval '1 microsecond')
            WHERE id = $1
              AND team_id = $2
              AND base_state = 'in_progress'
              AND updated_at = $3
            "#,
        )
        .bind(release_id)
        .bind(incident.team_id)
        .bind(expected_updated_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if locked.rows_affected() != 1 {
            return Err(DomainError::ConcurrentModification);
        }

        insert_incident(&mut transaction, incident).await?;
        insert_event(&mut transaction, event).await?;
        sqlx::query(
            r#"
            INSERT INTO release_incidents (team_id, release_id, incident_id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(incident.team_id)
        .bind(release_id)
        .bind(incident.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        transaction.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn find_release_by_id(&self, release_id: Uuid) -> Result<Option<Release>, DomainError> {
        let row = sqlx::query!(
            r#"SELECT id, team_id, title, base_state, created_at, updated_at FROM releases WHERE id = $1"#,
            release_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let steps = sqlx::query!(
            r#"
            SELECT position, name, validated_by, validated_at
            FROM release_steps
            WHERE release_id = $1
            ORDER BY position
            "#,
            release_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(Some(Release {
            id: row.id,
            team_id: row.team_id,
            title: row.title,
            base_state: ReleaseBaseState::try_from(row.base_state.as_str())?,
            steps: steps
                .into_iter()
                .map(|s| ReleaseStep {
                    position: s.position,
                    name: s.name,
                    validated_by: s.validated_by,
                    validated_at: s.validated_at,
                })
                .collect(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, team_id, title, base_state, created_at, updated_at
            FROM releases
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let step_rows = sqlx::query!(
            r#"
            SELECT release_id, position, name, validated_by, validated_at
            FROM release_steps
            WHERE release_id = ANY($1)
            ORDER BY release_id, position
            "#,
            &ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let mut steps_by_release: HashMap<Uuid, Vec<ReleaseStep>> = HashMap::new();
        for s in step_rows {
            steps_by_release
                .entry(s.release_id)
                .or_default()
                .push(ReleaseStep {
                    position: s.position,
                    name: s.name,
                    validated_by: s.validated_by,
                    validated_at: s.validated_at,
                });
        }

        rows.into_iter()
            .map(|row| {
                Ok(Release {
                    id: row.id,
                    team_id: row.team_id,
                    title: row.title,
                    base_state: ReleaseBaseState::try_from(row.base_state.as_str())?,
                    steps: steps_by_release.remove(&row.id).unwrap_or_default(),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn update_release(
        &self,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::update_release_rows(&mut tx, release, expected_updated_at).await?;
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn update_release_with_incident_events(
        &self,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
        events: &[IncidentEvent],
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::update_release_rows(&mut transaction, release, expected_updated_at).await?;
        for event in events {
            insert_event(&mut transaction, event).await?;
        }
        transaction.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn link_incident(&self, release_id: Uuid, incident_id: Uuid) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let linked = sqlx::query(
            r#"
            INSERT INTO release_incidents (team_id, release_id, incident_id)
            SELECT release.team_id, release.id, $2
            FROM releases release
            WHERE release.id = $1
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(release_id)
        .bind(incident_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if linked.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE releases SET updated_at = now() WHERE id = $1"#,
                release_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn unlink_incident(
        &self,
        release_id: Uuid,
        incident_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let unlinked = sqlx::query!(
            r#"DELETE FROM release_incidents WHERE release_id = $1 AND incident_id = $2"#,
            release_id,
            incident_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if unlinked.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE releases SET updated_at = now() WHERE id = $1"#,
                release_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn list_linked_incident_ids(&self, release_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query!(
            r#"SELECT incident_id FROM release_incidents WHERE release_id = $1"#,
            release_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(rows.into_iter().map(|r| r.incident_id).collect())
    }

    async fn count_active_linked_incidents(&self, release_id: Uuid) -> Result<u64, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM release_incidents ri
            JOIN incidents i ON i.id = ri.incident_id
            WHERE ri.release_id = $1 AND i.status <> 'resolved'
            "#,
            release_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(row.count as u64)
    }

    async fn list_release_states_linked_to_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState)>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT r.id, r.team_id, r.base_state
            FROM release_incidents ri
            JOIN releases r ON r.id = ri.release_id
            WHERE ri.incident_id = $1
            "#,
            incident_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    row.id,
                    row.team_id,
                    ReleaseBaseState::try_from(row.base_state.as_str())?,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "release_tests.rs"]
mod tests;
