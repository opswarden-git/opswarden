use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::ports::IncidentRepo;

pub struct PgIncidentRepo {
    pool: PgPool,
}

impl PgIncidentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        sqlx::query!(
            r#"
            INSERT INTO incidents (id, team_id, title, status, severity, assignee_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            incident.id,
            incident.team_id,
            incident.title,
            status_to_str(incident.status),
            severity_to_str(incident.severity),
            incident.assignee,
            incident.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn find_incident_by_id(
        &self,
        incident_id: Uuid,
    ) -> Result<Option<Incident>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, team_id, title, status, severity, assignee_id, created_at
            FROM incidents
            WHERE id = $1
            "#,
            incident_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.map(|row| Incident {
            id: row.id,
            team_id: row.team_id,
            title: row.title,
            status: status_from_str(&row.status),
            severity: severity_from_str(&row.severity),
            assignee: row.assignee_id,
            created_at: row.created_at,
        }))
    }

    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE incidents
            SET title = $2, status = $3, severity = $4, assignee_id = $5
            WHERE id = $1
            "#,
            incident.id,
            incident.title,
            status_to_str(incident.status),
            severity_to_str(incident.severity),
            incident.assignee,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT id, team_id, title, status, severity, assignee_id, created_at
            FROM incidents
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records
            .into_iter()
            .map(|row| Incident {
                id: row.id,
                team_id: row.team_id,
                title: row.title,
                status: status_from_str(&row.status),
                severity: severity_from_str(&row.severity),
                assignee: row.assignee_id,
                created_at: row.created_at,
            })
            .collect())
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
}
