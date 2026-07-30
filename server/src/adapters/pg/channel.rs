use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::channel::Channel;
use crate::domain::error::DomainError;
use crate::ports::ChannelRepo;

pub struct PgChannelRepo {
    pool: PgPool,
}

impl PgChannelRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChannelRepo for PgChannelRepo {
    async fn create_channel(&self, channel: &Channel) -> Result<(), DomainError> {
        let query = "
            insert into channels (id, team_id, name, created_at)
            values ($1, $2, $3, $4)
        ";
        sqlx::query(query)
            .bind(channel.id)
            .bind(channel.team_id)
            .bind(&channel.name)
            .bind(channel.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("23505") {
                        // Unique constraint violation (channel name in team)
                        return DomainError::InvalidChannelName;
                    }
                }
                DomainError::Storage
            })?;
        Ok(())
    }

    async fn list_channels_for_team(&self, team_id: Uuid) -> Result<Vec<Channel>, DomainError> {
        let records = sqlx::query_as::<_, Channel>(
            r#"
            select id, team_id, name, created_at
            from channels
            where team_id = $1
            order by created_at asc
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(records)
    }

    async fn find_channel_by_id(&self, channel_id: Uuid) -> Result<Option<Channel>, DomainError> {
        let record = sqlx::query_as::<_, Channel>(
            r#"
            select id, team_id, name, created_at
            from channels
            where id = $1
            "#
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(record)
    }

    async fn delete_channel(&self, team_id: Uuid, channel_id: Uuid) -> Result<(), DomainError> {
        let query = "
            delete from channels
            where team_id = $1 and id = $2
        ";
        let result = sqlx::query(query)
            .bind(team_id)
            .bind(channel_id)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::ChannelNotFound);
        }
        Ok(())
    }
}
