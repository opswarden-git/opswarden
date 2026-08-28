// --- server/src/adapters/pg/private_message.rs ---
//
// PostgreSQL adapter for bilateral messages, their bounded attachments and
// reactions. Conversation reads are keyset-paginated and hydrated in three
// bounded queries, avoiding one query per message.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::private_message::{
    PrivateMessage, PrivateMessageAttachment, PrivateMessageReaction,
};
use crate::ports::PrivateMessageRepo;

pub struct PgPrivateMessageRepo {
    pool: PgPool,
}

impl PgPrivateMessageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrivateMessageRepo for PgPrivateMessageRepo {
    async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        sqlx::query(
            "INSERT INTO private_messages \
             (id, sender_id, recipient_id, content, created_at, edited_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(message.id)
        .bind(message.sender_id)
        .bind(message.recipient_id)
        .bind(&message.content)
        .bind(message.created_at)
        .bind(message.edited_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        for attachment in &message.attachments {
            sqlx::query(
                "INSERT INTO private_message_attachments \
                 (id, message_id, file_name, media_type, content, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(attachment.id)
            .bind(message.id)
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

    async fn list_conversation(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        before: Option<(DateTime<Utc>, Uuid)>,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError> {
        let (before_at, before_id) = before.unzip();
        let rows = sqlx::query(
            r#"
            SELECT id, sender_id, recipient_id, content, created_at, edited_at
            FROM private_messages
            WHERE ((sender_id = $1 AND recipient_id = $2)
                OR (sender_id = $2 AND recipient_id = $1))
              AND ($3::timestamptz IS NULL OR (created_at, id) < ($3, $4))
            ORDER BY created_at DESC, id DESC
            LIMIT $5
            "#,
        )
        .bind(viewer_id)
        .bind(peer_id)
        .bind(before_at)
        .bind(before_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let mut messages = rows
            .into_iter()
            .map(|row| PrivateMessage {
                id: row.get("id"),
                sender_id: row.get("sender_id"),
                recipient_id: row.get("recipient_id"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                edited_at: row.get("edited_at"),
                attachments: Vec::new(),
                reactions: Vec::new(),
            })
            .collect::<Vec<_>>();
        let message_ids = messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>();
        if message_ids.is_empty() {
            return Ok(messages);
        }

        let attachment_rows = sqlx::query(
            "SELECT id, message_id, file_name, media_type, octet_length(content)::bigint AS size_bytes, created_at \
             FROM private_message_attachments WHERE message_id = ANY($1) \
             ORDER BY created_at, id",
        )
        .bind(&message_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        for row in attachment_rows {
            let message_id: Uuid = row.get("message_id");
            if let Some(message) = messages.iter_mut().find(|message| message.id == message_id) {
                message.attachments.push(PrivateMessageAttachment {
                    id: row.get("id"),
                    message_id,
                    file_name: row.get("file_name"),
                    media_type: row.get("media_type"),
                    size_bytes: row.get::<i64, _>("size_bytes") as usize,
                    content: Vec::new(),
                    created_at: row.get("created_at"),
                });
            }
        }

        let reaction_rows = sqlx::query(
            "SELECT message_id, emoji, count(*)::bigint AS count, \
                    bool_or(user_id = $2) AS reacted \
             FROM private_message_reactions WHERE message_id = ANY($1) \
             GROUP BY message_id, emoji ORDER BY min(created_at), emoji",
        )
        .bind(&message_ids)
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        for row in reaction_rows {
            let message_id: Uuid = row.get("message_id");
            if let Some(message) = messages.iter_mut().find(|message| message.id == message_id) {
                let count: i64 = row.get("count");
                message.reactions.push(PrivateMessageReaction {
                    emoji: row.get("emoji"),
                    count: count as u64,
                    reacted: row.get("reacted"),
                });
            }
        }

        Ok(messages)
    }

    async fn find_participants(
        &self,
        message_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, DomainError> {
        sqlx::query!(
            "SELECT sender_id, recipient_id FROM private_messages WHERE id = $1",
            message_id
        )
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(|row| (row.sender_id, row.recipient_id)))
        .map_err(|_| DomainError::Storage)
    }

    async fn update_content(
        &self,
        message_id: Uuid,
        content: &str,
        edited_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE private_messages SET content = $2, edited_at = $3 WHERE id = $1")
            .bind(message_id)
            .bind(content)
            .bind(edited_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|_| DomainError::Storage)
    }

    async fn toggle_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let removed = sqlx::query(
            "DELETE FROM private_message_reactions \
             WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(emoji)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?
        .rows_affected()
            > 0;
        if !removed {
            sqlx::query(
                "INSERT INTO private_message_reactions \
                 (message_id, user_id, emoji, created_at) VALUES ($1, $2, $3, $4)",
            )
            .bind(message_id)
            .bind(user_id)
            .bind(emoji)
            .bind(Utc::now())
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(!removed)
    }

    async fn find_attachment_for_participant(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PrivateMessageAttachment>, DomainError> {
        sqlx::query!(
            "SELECT a.id, a.message_id, a.file_name, a.media_type, a.content, a.created_at \
             FROM private_message_attachments a \
             JOIN private_messages m ON m.id = a.message_id \
             WHERE a.id = $1 AND (m.sender_id = $2 OR m.recipient_id = $2)",
            attachment_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map(|row| {
            row.map(|row| {
                let content = row.content;
                PrivateMessageAttachment {
                    id: row.id,
                    message_id: row.message_id,
                    file_name: row.file_name,
                    media_type: row.media_type,
                    size_bytes: content.len(),
                    content,
                    created_at: row.created_at,
                }
            })
        })
        .map_err(|_| DomainError::Storage)
    }

    async fn mark_read(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        read_through: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO private_message_reads (viewer_id, peer_id, read_through) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (viewer_id, peer_id) \
             DO UPDATE SET read_through = GREATEST(private_message_reads.read_through, EXCLUDED.read_through)",
        )
        .bind(viewer_id)
        .bind(peer_id)
        .bind(read_through)
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn list_unread_peer_ids(&self, viewer_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query(
            "SELECT DISTINCT m.sender_id \
             FROM private_messages m \
             LEFT JOIN private_message_reads r \
               ON r.viewer_id = m.recipient_id AND r.peer_id = m.sender_id \
             WHERE m.recipient_id = $1 \
               AND (r.read_through IS NULL OR m.created_at > r.read_through)",
        )
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(rows.into_iter().map(|row| row.get("sender_id")).collect())
    }
}

#[cfg(test)]
#[path = "private_message_tests.rs"]
mod tests;
