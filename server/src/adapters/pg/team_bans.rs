use super::*;

impl PgTeamRepo {
    pub(crate) async fn add_ban_impl(&self, ban: &TeamBan) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO team_bans (team_id, user_id, expires_at, reason, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (team_id, user_id) DO UPDATE
            SET expires_at = EXCLUDED.expires_at,
                reason     = EXCLUDED.reason,
                created_by = EXCLUDED.created_by,
                created_at = EXCLUDED.created_at
            "#,
            ban.team_id,
            ban.user_id,
            ban.expires_at(),
            ban.reason.as_deref(),
            ban.created_by,
            ban.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    pub(crate) async fn ban_member_and_clear_assignments_impl(
        &self,
        ban: &TeamBan,
        requester_id: Uuid,
    ) -> Result<bool, DomainError> {
        if ban.created_by != Some(requester_id) {
            return Err(DomainError::Forbidden);
        }
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let target_role =
            lock_moderation_target(&mut tx, ban.team_id, requester_id, ban.user_id).await?;
        sqlx::query(
            r#"
            INSERT INTO team_bans (team_id, user_id, expires_at, reason, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (team_id, user_id) DO UPDATE
            SET expires_at = EXCLUDED.expires_at,
                reason = EXCLUDED.reason,
                created_by = EXCLUDED.created_by,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(ban.team_id)
        .bind(ban.user_id)
        .bind(ban.expires_at())
        .bind(ban.reason.as_deref())
        .bind(ban.created_by)
        .bind(ban.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        let removed_membership = target_role.is_some();
        if removed_membership {
            let removed = sqlx::query(
                "DELETE FROM team_members WHERE team_id = $1 AND user_id = $2 AND role <> 'manager'",
            )
            .bind(ban.team_id)
            .bind(ban.user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
            if removed.rows_affected() != 1 {
                return Err(DomainError::MemberNotFound);
            }
            clear_member_assignments(&mut tx, ban.team_id, ban.user_id).await?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(removed_membership)
    }

    pub(crate) async fn find_ban_impl(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamBan>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT expires_at, reason, created_by, created_at
            FROM team_bans
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(row.map(|r| TeamBan {
            team_id,
            user_id,
            kind: ban_kind(r.expires_at),
            reason: r.reason,
            created_by: r.created_by,
            created_at: r.created_at,
        }))
    }

    pub(crate) async fn list_bans_impl(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<TeamBanView>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                bans.user_id,
                banned.email AS user_email,
                bans.expires_at,
                bans.reason,
                bans.created_by,
                moderator.email AS "moderator_email?",
                bans.created_at
            FROM team_bans bans
            JOIN users banned ON banned.id = bans.user_id
            LEFT JOIN users moderator ON moderator.id = bans.created_by
            WHERE bans.team_id = $1
            ORDER BY bans.created_at DESC
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(rows
            .into_iter()
            .map(|r| TeamBanView {
                ban: TeamBan {
                    team_id,
                    user_id: r.user_id,
                    kind: ban_kind(r.expires_at),
                    reason: r.reason,
                    created_by: r.created_by,
                    created_at: r.created_at,
                },
                user_email: r.user_email,
                moderator_email: r.moderator_email,
            })
            .collect())
    }

    pub(crate) async fn remove_ban_impl(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM team_bans
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    pub(crate) async fn save_team_image_impl(
        &self,
        team_id: Uuid,
        image: &TeamImage,
    ) -> Result<(), DomainError> {
        team_image::save(&self.pool, team_id, image).await
    }

    pub(crate) async fn find_team_image_for_member_impl(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamImage>, DomainError> {
        team_image::find_for_member(&self.pool, team_id, user_id).await
    }

    pub(crate) async fn delete_team_image_impl(&self, team_id: Uuid) -> Result<(), DomainError> {
        team_image::delete(&self.pool, team_id).await
    }
}
