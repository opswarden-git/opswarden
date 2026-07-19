use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::incident_event::{IncidentEvent, IncidentEventKind};
use crate::ports::IncidentRepo;

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

impl From<IncidentRow> for Incident {
    fn from(row: IncidentRow) -> Self {
        Self {
            id: row.id,
            team_id: row.team_id,
            title: row.title,
            description: row.description,
            status: status_from_str(&row.status),
            severity: severity_from_str(&row.severity),
            assignee: row.assignee_id,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
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
        _ => None,
    }
}

fn status_to_str(status: IncidentStatus) -> &'static str {
    match status {
        IncidentStatus::Open => "open",
        IncidentStatus::Acknowledged => "acknowledged",
        IncidentStatus::Escalated => "escalated",
        IncidentStatus::Resolved => "resolved",
    }
}

fn status_from_str(value: &str) -> IncidentStatus {
    match value {
        "acknowledged" => IncidentStatus::Acknowledged,
        "escalated" => IncidentStatus::Escalated,
        "resolved" => IncidentStatus::Resolved,
        _ => IncidentStatus::Open,
    }
}

fn severity_to_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "medium" => Severity::Medium,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => Severity::Low,
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
        .bind(status_to_str(incident.status))
        .bind(severity_to_str(incident.severity))
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
        .bind(status_to_str(incident.status))
        .bind(severity_to_str(incident.severity))
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

        Ok(record.map(Incident::from))
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
        .bind(status_to_str(incident.status))
        .bind(severity_to_str(incident.severity))
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
        .bind(status_to_str(incident.status))
        .bind(severity_to_str(incident.severity))
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

    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError> {
        let records = sqlx::query_as::<_, IncidentEventRow>(
            r#"
            SELECT id, incident_id, kind, actor_id, data, created_at
            FROM incident_events
            WHERE incident_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(incident_id)
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

        Ok(records.into_iter().map(Incident::from).collect())
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
mod tests {
    use super::*;
    use crate::adapters::pg::team::PgTeamRepo;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::team::{Role, Team};
    use crate::domain::user::{Email, User};
    use crate::ports::{TeamRepo, UserRepo};
    async fn seed_team(pool: &PgPool) -> Uuid {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let email = Email::new(format!("incident_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();

        let team = Team::new("Incident Team").unwrap();
        teams.save_team(&team).await.unwrap();
        teams
            .add_member(team.id, user.id, Role::Manager)
            .await
            .unwrap();
        team.id
    }

    #[sqlx::test]
    async fn it_saves_finds_and_updates_incidents_in_postgres(pool: PgPool) {
        let repo = PgIncidentRepo::new(pool.clone());
        let team_id = seed_team(&pool).await;

        let mut incident = Incident::new(team_id, "API saturation", Severity::High).unwrap();
        repo.save_incident(&incident).await.unwrap();

        let found = repo
            .find_incident_by_id(incident.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, "API saturation");
        assert_eq!(found.status, IncidentStatus::Open);

        incident.acknowledge().unwrap();
        repo.update_incident(&incident).await.unwrap();

        let updated = repo
            .find_incident_by_id(incident.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, IncidentStatus::Acknowledged);
    }

    #[sqlx::test]
    async fn clear_assignee_for_member_unassigns_their_incidents(pool: PgPool) {
        let repo = PgIncidentRepo::new(pool.clone());
        let team_id = seed_team(&pool).await;

        // A user to assign, then "remove" from the team.
        let users = PgUserRepo::new(pool.clone());
        let email = Email::new(format!("assignee_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let assignee = User::new(email, "hash");
        users.save(&assignee).await.unwrap();

        let mut incident = Incident::new(team_id, "owned by a member", Severity::High).unwrap();
        incident.assign(assignee.id);
        repo.save_incident(&incident).await.unwrap();
        assert_eq!(
            repo.find_incident_by_id(incident.id)
                .await
                .unwrap()
                .unwrap()
                .assignee,
            Some(assignee.id)
        );

        repo.clear_assignee_for_member(team_id, assignee.id)
            .await
            .unwrap();

        assert_eq!(
            repo.find_incident_by_id(incident.id)
                .await
                .unwrap()
                .unwrap()
                .assignee,
            None
        );
    }

    #[sqlx::test]
    async fn incident_and_initial_event_are_committed_together(pool: PgPool) {
        let repo = PgIncidentRepo::new(pool.clone());
        let team_id = seed_team(&pool).await;
        let incident = Incident::new(team_id, "API saturation", Severity::Critical).unwrap();
        let event = IncidentEvent::created(&incident, None);

        repo.save_incident_with_event(&incident, &event)
            .await
            .unwrap();

        let events = repo
            .list_events_for_incident(incident.id, 10)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, IncidentEventKind::Created);
    }

    #[sqlx::test]
    async fn a_failed_event_rolls_back_the_incident_write(pool: PgPool) {
        let repo = PgIncidentRepo::new(pool.clone());
        let team_id = seed_team(&pool).await;
        let incident = Incident::new(team_id, "Must roll back", Severity::High).unwrap();
        let mut invalid_event = IncidentEvent::created(&incident, None);
        invalid_event.incident_id = Uuid::new_v4();

        assert_eq!(
            repo.save_incident_with_event(&incident, &invalid_event)
                .await
                .unwrap_err(),
            DomainError::Storage
        );
        assert!(repo
            .find_incident_by_id(incident.id)
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn a_failed_event_rolls_back_the_incident_update(pool: PgPool) {
        let repo = PgIncidentRepo::new(pool.clone());
        let team_id = seed_team(&pool).await;
        let mut incident = Incident::new(team_id, "Stable state", Severity::High).unwrap();
        repo.save_incident(&incident).await.unwrap();
        incident.acknowledge().unwrap();
        let invalid_event = IncidentEvent::status_changed(
            Uuid::new_v4(),
            Uuid::new_v4(),
            IncidentStatus::Open,
            IncidentStatus::Acknowledged,
        );

        assert_eq!(
            repo.update_incident_with_event(&incident, &invalid_event)
                .await
                .unwrap_err(),
            DomainError::Storage
        );
        assert_eq!(
            repo.find_incident_by_id(incident.id)
                .await
                .unwrap()
                .unwrap()
                .status,
            IncidentStatus::Open
        );
    }
}
