use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::incident_event::{IncidentEvent, IncidentEventKind};
use crate::ports::{ActivityCursor, IncidentRepo};

pub struct PgIncidentRepo {
    pool: PgPool,
}

impl PgIncidentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct IncidentRow {
    id: Uuid,
    team_id: Uuid,
    title: String,
    description: String,
    status: String,
    severity: String,
    assignee_id: Option<Uuid>,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<IncidentRow> for Incident {
    type Error = DomainError;

    fn try_from(row: IncidentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            team_id: row.team_id,
            title: row.title,
            description: row.description,
            status: IncidentStatus::try_from(row.status.as_str())
                .map_err(|_| DomainError::Storage)?,
            severity: Severity::try_from(row.severity.as_str())
                .map_err(|_| DomainError::Storage)?,
            assignee: row.assignee_id,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(FromRow)]
struct IncidentEventRow {
    id: Uuid,
    incident_id: Uuid,
    kind: String,
    actor_id: Option<Uuid>,
    data: Value,
    created_at: DateTime<Utc>,
}

fn event_kind_from_str(value: &str) -> Option<IncidentEventKind> {
    match value {
        "created" => Some(IncidentEventKind::Created),
        "status_changed" => Some(IncidentEventKind::StatusChanged),
        "assigned" => Some(IncidentEventKind::Assigned),
        "severity_changed" => Some(IncidentEventKind::SeverityChanged),
        "release_step_validated" => Some(IncidentEventKind::ReleaseStepValidated),
        _ => None,
    }
}

#[async_trait]
impl IncidentRepo for PgIncidentRepo {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO incidents (
                id, team_id, title, description, status, severity, assignee_id,
                created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(incident.id)
        .bind(incident.team_id)
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.status.as_str())
        .bind(incident.severity.as_str())
        .bind(incident.assignee)
        .bind(incident.created_by)
        .bind(incident.created_at)
        .bind(incident.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn save_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        sqlx::query(
            r#"
            INSERT INTO incidents (
                id, team_id, title, description, status, severity, assignee_id,
                created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(incident.id)
        .bind(incident.team_id)
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.status.as_str())
        .bind(incident.severity.as_str())
        .bind(incident.assignee)
        .bind(incident.created_by)
        .bind(incident.created_at)
        .bind(incident.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query(
            r#"
            INSERT INTO incident_events (id, incident_id, kind, actor_id, data, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(event.id)
        .bind(event.incident_id)
        .bind(event.kind.to_string())
        .bind(event.actor_id)
        .bind(&event.data)
        .bind(event.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn find_incident_by_id(
        &self,
        incident_id: Uuid,
    ) -> Result<Option<Incident>, DomainError> {
        let record = sqlx::query_as::<_, IncidentRow>(
            r#"
            SELECT id, team_id, title, description, status, severity,
                   assignee_id, created_by, created_at, updated_at
            FROM incidents
            WHERE id = $1
            "#,
        )
        .bind(incident_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        record.map(Incident::try_from).transpose()
    }

    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE incidents
            SET title = $2, description = $3, status = $4, severity = $5,
                assignee_id = $6, updated_at = $7
            WHERE id = $1
            "#,
        )
        .bind(incident.id)
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.status.as_str())
        .bind(incident.severity.as_str())
        .bind(incident.assignee)
        .bind(incident.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn update_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        sqlx::query(
            r#"
            UPDATE incidents
            SET title = $2, description = $3, status = $4, severity = $5,
                assignee_id = $6, updated_at = $7
            WHERE id = $1
            "#,
        )
        .bind(incident.id)
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.status.as_str())
        .bind(incident.severity.as_str())
        .bind(incident.assignee)
        .bind(incident.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query(
            r#"
            INSERT INTO incident_events (id, incident_id, kind, actor_id, data, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(event.id)
        .bind(event.incident_id)
        .bind(event.kind.to_string())
        .bind(event.actor_id)
        .bind(&event.data)
        .bind(event.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn record_events(&self, events: &[IncidentEvent]) -> Result<(), DomainError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        for event in events {
            sqlx::query(
                r#"
                INSERT INTO incident_events (id, incident_id, kind, actor_id, data, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(event.id)
            .bind(event.incident_id)
            .bind(event.kind.to_string())
            .bind(event.actor_id)
            .bind(&event.data)
            .bind(event.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError> {
        let (before_at, before_id) = before.unzip();
        let records = sqlx::query_as::<_, IncidentEventRow>(
            r#"
            SELECT id, incident_id, kind, actor_id, data, created_at
            FROM incident_events
            WHERE incident_id = $1
              AND ($2::timestamptz IS NULL OR (created_at, id) < ($2, $3))
            ORDER BY created_at DESC, id DESC
            LIMIT $4
            "#,
        )
        .bind(incident_id)
        .bind(before_at)
        .bind(before_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        records
            .into_iter()
            .map(|row| {
                Ok(IncidentEvent {
                    id: row.id,
                    incident_id: row.incident_id,
                    kind: event_kind_from_str(&row.kind).ok_or(DomainError::Storage)?,
                    actor_id: row.actor_id,
                    data: row.data,
                    created_at: row.created_at,
                })
            })
            .collect()
    }

    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError> {
        let records = sqlx::query_as::<_, IncidentRow>(
            r#"
            SELECT id, team_id, title, description, status, severity,
                   assignee_id, created_by, created_at, updated_at
            FROM incidents
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        records.into_iter().map(Incident::try_from).collect()
    }

    async fn list_unread_incident_ids(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, DomainError> {
        sqlx::query_scalar(
            r#"
            SELECT incident.id
            FROM incidents incident
            LEFT JOIN incident_channel_reads channel_read
              ON channel_read.incident_id = incident.id
             AND channel_read.user_id = $2
            WHERE incident.team_id = $1
              AND (
                EXISTS (
                  SELECT 1
                  FROM timeline_entries entry
                  WHERE entry.incident_id = incident.id
                    AND entry.author_id IS DISTINCT FROM $2
                    AND entry.created_at > COALESCE(channel_read.read_through, '-infinity')
                )
                OR EXISTS (
                  SELECT 1
                  FROM incident_events event
                  WHERE event.incident_id = incident.id
                    AND event.actor_id IS DISTINCT FROM $2
                    AND event.created_at > COALESCE(channel_read.read_through, '-infinity')
                )
              )
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)
    }

    async fn mark_incident_read(
        &self,
        incident_id: Uuid,
        user_id: Uuid,
        read_through: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO incident_channel_reads (incident_id, user_id, read_through)
            VALUES ($1, $2, LEAST($3, now()))
            ON CONFLICT (incident_id, user_id) DO UPDATE
            SET read_through = GREATEST(
              incident_channel_reads.read_through,
              EXCLUDED.read_through
            )
            "#,
        )
        .bind(incident_id)
        .bind(user_id)
        .bind(read_through)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM incidents
            WHERE id = $1
            "#,
            incident_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn clear_assignee_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE incidents SET assignee_id = NULL
            WHERE team_id = $1 AND assignee_id = $2
            "#,
            team_id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "incident_tests.rs"]
mod tests;
