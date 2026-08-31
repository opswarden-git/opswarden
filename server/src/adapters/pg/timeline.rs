use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::conversation::MessageAttachment;
use crate::domain::error::DomainError;
use crate::domain::timeline::{ReactionRecord, TimelineEntry};
use crate::ports::{ActivityCursor, TimelineRepo};

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
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
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
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        for attachment in &entry.attachments {
            sqlx::query(
                "INSERT INTO timeline_entry_attachments \
                 (id, entry_id, file_name, media_type, content, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(attachment.id)
            .bind(entry.id)
            .bind(&attachment.file_name)
            .bind(&attachment.media_type)
            .bind(&attachment.content)
            .bind(attachment.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }

        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError> {
        let (before_at, before_id) = before.unzip();
        // `id` joins the ordering as well as the predicate: without it two notes
        // written in the same millisecond could straddle a page boundary and be
        // returned twice, or not at all.
        let records = sqlx::query!(
            r#"
            SELECT id, incident_id, author_id, content, created_at, edited_at
            FROM timeline_entries
            WHERE incident_id = $1
              AND ($2::timestamptz IS NULL OR (created_at, id) < ($2, $3))
            ORDER BY created_at DESC, id DESC
            LIMIT $4
            "#,
            incident_id,
            before_at,
            before_id,
            i64::from(limit),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let mut entries = records
            .into_iter()
            .map(|row| TimelineEntry {
                id: row.id,
                incident_id: row.incident_id,
                author_id: row.author_id,
                content: row.content,
                created_at: row.created_at,
                edited_at: row.edited_at,
                attachments: Vec::new(),
            })
            .collect::<Vec<_>>();
        let entry_ids = entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
        if !entry_ids.is_empty() {
            let rows = sqlx::query(
                "SELECT id, entry_id, file_name, media_type, \
                 octet_length(content)::bigint AS size_bytes, created_at \
                 FROM timeline_entry_attachments WHERE entry_id = ANY($1) \
                 ORDER BY created_at, id",
            )
            .bind(&entry_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
            for row in rows {
                let entry_id: Uuid = row.get("entry_id");
                if let Some(entry) = entries.iter_mut().find(|entry| entry.id == entry_id) {
                    entry.attachments.push(MessageAttachment {
                        id: row.get("id"),
                        message_id: entry_id,
                        file_name: row.get("file_name"),
                        media_type: row.get("media_type"),
                        size_bytes: row.get::<i64, _>("size_bytes") as usize,
                        content: Vec::new(),
                        created_at: row.get("created_at"),
                    });
                }
            }
        }
        Ok(entries)
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
            attachments: Vec::new(),
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

    async fn find_attachment_for_member(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MessageAttachment>, DomainError> {
        let row = sqlx::query!(
            "SELECT a.id, a.entry_id, a.file_name, a.media_type, a.content, a.created_at \
             FROM timeline_entry_attachments a \
             JOIN timeline_entries e ON e.id = a.entry_id \
             JOIN incidents i ON i.id = e.incident_id \
             JOIN team_members m ON m.team_id = i.team_id AND m.user_id = $2 \
             WHERE a.id = $1",
            attachment_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(row.map(|row| {
            let content = row.content;
            MessageAttachment {
                id: row.id,
                message_id: row.entry_id,
                file_name: row.file_name,
                media_type: row.media_type,
                size_bytes: content.len(),
                content,
                created_at: row.created_at,
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::incident::PgIncidentRepo;
    use crate::adapters::pg::team::PgTeamRepo;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::Team;
    use crate::domain::timeline::TimelineEntry;
    use crate::domain::user::{Email, User};
    use crate::ports::{IncidentRepo, TeamRepo, UserRepo};
    async fn seed_incident(pool: &PgPool) -> (Uuid, Uuid) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let incidents = PgIncidentRepo::new(pool.clone());

        let email = Email::new(format!("timeline_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();

        let team = Team::new("Timeline Team").unwrap();
        teams
            .create_team_with_manager(&team, user.id)
            .await
            .unwrap();

        let incident = Incident::new(team.id, "Ingress instability", Severity::High).unwrap();
        incidents.save_incident(&incident).await.unwrap();

        (incident.id, user.id)
    }

    #[sqlx::test]
    async fn it_appends_and_lists_recent_entries_in_postgres(pool: PgPool) {
        let repo = PgTimelineRepo::new(pool.clone());
        let (incident_id, author_id) = seed_incident(&pool).await;

        let first = TimelineEntry::new(incident_id, author_id, "Checking logs").unwrap();
        let second = TimelineEntry::new(incident_id, author_id, "Issue isolated").unwrap();
        repo.append_entry(&first).await.unwrap();
        repo.append_entry(&second).await.unwrap();

        let entries = repo
            .list_entries_for_incident(incident_id, None, 1)
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Issue isolated");
    }

    #[sqlx::test]
    async fn incident_attachments_are_atomic_and_team_authorized(pool: PgPool) {
        let repo = PgTimelineRepo::new(pool.clone());
        let (incident_id, author_id) = seed_incident(&pool).await;
        let entry = TimelineEntry::new_with_attachments(
            incident_id,
            author_id,
            "",
            vec![("runbook.txt".into(), "text/plain".into(), b"steps".to_vec())],
        )
        .unwrap();
        let attachment_id = entry.attachments[0].id;
        repo.append_entry(&entry).await.unwrap();

        let listed = repo
            .list_entries_for_incident(incident_id, None, 10)
            .await
            .unwrap();
        assert_eq!(listed[0].attachments[0].file_name, "runbook.txt");
        assert_eq!(listed[0].attachments[0].size_bytes, 5);
        assert_eq!(
            repo.find_attachment_for_member(attachment_id, author_id)
                .await
                .unwrap()
                .unwrap()
                .content,
            b"steps"
        );
        assert!(repo
            .find_attachment_for_member(attachment_id, Uuid::new_v4())
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn it_edits_an_entry_keeping_created_at_in_postgres(pool: PgPool) {
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

    #[sqlx::test]
    async fn it_toggles_and_counts_reactions_without_duplicates_in_postgres(pool: PgPool) {
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
