use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::timeline::TimelineEntry;
use crate::ports::TimelineRepo;

pub struct PgTimelineRepo {
    pool: PgPool,
}

impl PgTimelineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TimelineRepo for PgTimelineRepo {
    async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO timeline_entries (id, incident_id, author_id, content, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            entry.id,
            entry.incident_id,
            entry.author_id,
            entry.content,
            entry.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT id, incident_id, author_id, content, created_at
            FROM timeline_entries
            WHERE incident_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            incident_id,
            i64::from(limit as i32),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records
            .into_iter()
            .map(|row| TimelineEntry {
                id: row.id,
                incident_id: row.incident_id,
                author_id: row.author_id,
                content: row.content,
                created_at: row.created_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::incident::PgIncidentRepo;
    use crate::adapters::pg::team::PgTeamRepo;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::{Role, Team};
    use crate::domain::timeline::TimelineEntry;
    use crate::domain::user::{Email, User};
    use crate::ports::{IncidentRepo, TeamRepo, UserRepo};
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string()
        });
        PgPoolOptions::new().connect(&database_url).await.unwrap()
    }

    async fn seed_incident(pool: &PgPool) -> (Uuid, Uuid) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let incidents = PgIncidentRepo::new(pool.clone());

        let email = Email::new(format!("timeline_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();

        let team = Team::new("Timeline Team").unwrap();
        teams.save_team(&team).await.unwrap();
        teams
            .add_member(team.id, user.id, Role::Manager)
            .await
            .unwrap();

        let incident = Incident::new(team.id, "Ingress instability", Severity::High).unwrap();
        incidents.save_incident(&incident).await.unwrap();

        (incident.id, user.id)
    }

    #[tokio::test]
    async fn it_appends_and_lists_recent_entries_in_postgres() {
        let pool = test_pool().await;
        let repo = PgTimelineRepo::new(pool.clone());
        let (incident_id, author_id) = seed_incident(&pool).await;

        let first = TimelineEntry::new(incident_id, author_id, "Checking logs").unwrap();
        let second = TimelineEntry::new(incident_id, author_id, "Issue isolated").unwrap();
        repo.append_entry(&first).await.unwrap();
        repo.append_entry(&second).await.unwrap();

        let entries = repo
            .list_entries_for_incident(incident_id, 1)
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Issue isolated");
    }
}
