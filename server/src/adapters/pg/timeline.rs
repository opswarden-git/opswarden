use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::timeline::{ReactionRecord, TimelineEntry};
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
            SELECT id, incident_id, author_id, content, created_at, edited_at
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
                edited_at: row.edited_at,
            })
            .collect())
    }

    async fn find_entry_by_id(&self, entry_id: Uuid) -> Result<Option<TimelineEntry>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, incident_id, author_id, content, created_at, edited_at
            FROM timeline_entries
            WHERE id = $1
            "#,
            entry_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.map(|row| TimelineEntry {
            id: row.id,
            incident_id: row.incident_id,
            author_id: row.author_id,
            content: row.content,
            created_at: row.created_at,
            edited_at: row.edited_at,
        }))
    }

    async fn update_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE timeline_entries SET content = $2, edited_at = $3
            WHERE id = $1
            "#,
            entry.id,
            entry.content,
            entry.edited_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn add_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO timeline_reactions (entry_id, user_id, emoji)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            entry_id,
            user_id,
            emoji,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(result.rows_affected() > 0)
    }

    async fn remove_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM timeline_reactions
            WHERE entry_id = $1 AND user_id = $2 AND emoji = $3
            "#,
            entry_id,
            user_id,
            emoji,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn count_reaction(&self, entry_id: Uuid, emoji: &str) -> Result<u64, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM timeline_reactions
            WHERE entry_id = $1 AND emoji = $2
            "#,
            entry_id,
            emoji,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.count as u64)
    }

    async fn list_reactions_for_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<ReactionRecord>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT r.entry_id, r.user_id, r.emoji
            FROM timeline_reactions r
            JOIN timeline_entries e ON e.id = r.entry_id
            WHERE e.incident_id = $1
            "#,
            incident_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records
            .into_iter()
            .map(|row| ReactionRecord {
                entry_id: row.entry_id,
                user_id: row.user_id,
                emoji: row.emoji,
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

    #[tokio::test]
    async fn it_edits_an_entry_keeping_created_at_in_postgres() {
        let pool = test_pool().await;
        let repo = PgTimelineRepo::new(pool.clone());
        let (incident_id, author_id) = seed_incident(&pool).await;

        let mut entry = TimelineEntry::new(incident_id, author_id, "before").unwrap();
        repo.append_entry(&entry).await.unwrap();
        // Read back the stored created_at (Postgres truncates to microseconds, so
        // compare DB value to DB value rather than to the in-memory nanoseconds).
        let stored_created_at = repo
            .find_entry_by_id(entry.id)
            .await
            .unwrap()
            .unwrap()
            .created_at;

        entry.edit("after").unwrap();
        repo.update_entry(&entry).await.unwrap();

        let loaded = repo.find_entry_by_id(entry.id).await.unwrap().unwrap();
        assert_eq!(loaded.content, "after");
        assert!(loaded.edited_at.is_some());
        assert_eq!(loaded.created_at, stored_created_at);
    }

    #[tokio::test]
    async fn it_toggles_and_counts_reactions_without_duplicates_in_postgres() {
        let pool = test_pool().await;
        let repo = PgTimelineRepo::new(pool.clone());
        let (incident_id, author_id) = seed_incident(&pool).await;

        let entry = TimelineEntry::new(incident_id, author_id, "react to me").unwrap();
        repo.append_entry(&entry).await.unwrap();

        assert!(repo.add_reaction(entry.id, author_id, "👍").await.unwrap());
        // Same user + emoji again: not newly inserted, and no duplicate row.
        assert!(!repo.add_reaction(entry.id, author_id, "👍").await.unwrap());
        assert_eq!(repo.count_reaction(entry.id, "👍").await.unwrap(), 1);

        let listed = repo.list_reactions_for_incident(incident_id).await.unwrap();
        assert_eq!(listed.iter().filter(|r| r.entry_id == entry.id).count(), 1);

        repo.remove_reaction(entry.id, author_id, "👍")
            .await
            .unwrap();
        assert_eq!(repo.count_reaction(entry.id, "👍").await.unwrap(), 0);
        // Idempotent removal of a missing reaction.
        repo.remove_reaction(entry.id, author_id, "👍")
            .await
            .unwrap();
    }
}
