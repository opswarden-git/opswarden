// --- server/src/adapters/pg/team.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::{
    InvitationCode, Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamImage, TeamMemberView,
};
use crate::ports::TeamRepo;
use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[path = "team_bans.rs"]
mod team_bans;

#[path = "team_directory.rs"]
mod team_directory;

#[path = "team_image.rs"]
mod team_image;

#[path = "team_mapping.rs"]
mod team_mapping;

use team_mapping::{ban_kind, role_from_str};

pub struct PgTeamRepo {
    pool: PgPool,
}

impl PgTeamRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn lock_moderation_target(
    tx: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    requester_id: Uuid,
    target_user_id: Uuid,
) -> Result<Option<Role>, DomainError> {
    let requester = sqlx::query_scalar::<_, String>(
        "SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(team_id)
    .bind(requester_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| DomainError::Storage)?;
    match requester.as_deref() {
        None => return Err(DomainError::Forbidden),
        Some(role) if role != Role::Manager.as_str() => return Err(DomainError::NotManager),
        Some(_) => {}
    }
    if requester_id == target_user_id {
        return Err(DomainError::CannotModerateSelf);
    }
    let target = sqlx::query_scalar::<_, String>(
        "SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(team_id)
    .bind(target_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| DomainError::Storage)?;
    let target = target.as_deref().map(role_from_str).transpose()?;
    if target == Some(Role::Manager) {
        return Err(DomainError::CannotModerateManager);
    }
    Ok(target)
}

async fn clear_member_assignments(
    tx: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<(), DomainError> {
    sqlx::query("UPDATE incidents SET assignee_id = NULL WHERE team_id = $1 AND assignee_id = $2")
        .bind(team_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| DomainError::Storage)?;
    Ok(())
}

#[async_trait]
impl TeamRepo for PgTeamRepo {
    async fn create_team_with_manager(
        &self,
        team: &Team,
        manager_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        sqlx::query(
            "INSERT INTO teams (id, name, invitation_code, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(team.id)
        .bind(&team.name)
        .bind(team.invitation_code.as_str())
        .bind(team.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'manager')")
            .bind(team.id)
            .bind(manager_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;

        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, name, invitation_code, created_at
            FROM teams
            WHERE invitation_code = $1
            "#,
            code,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.map(|row| Team {
            id: row.id,
            name: row.name,
            invitation_code: InvitationCode::from_existing(row.invitation_code),
            created_at: row.created_at,
        }))
    }

    async fn find_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT role
            FROM team_members
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        record.map(|row| role_from_str(&row.role)).transpose()
    }

    async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            "#,
            team_id,
            user_id,
            role.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            // PK clash (already a member) or the single-Manager partial index.
            sqlx::Error::Database(db) if db.is_unique_violation() => DomainError::AlreadyMember,
            _ => DomainError::Storage,
        })?;

        Ok(())
    }

    async fn transfer_manager(
        &self,
        team_id: Uuid,
        old_manager: Uuid,
        new_manager: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        let current_role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(team_id)
        .bind(old_manager)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if current_role.as_deref() != Some(Role::Manager.as_str()) {
            return Err(DomainError::NotManager);
        }

        let target_role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(team_id)
        .bind(new_manager)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if target_role.is_none() {
            return Err(DomainError::MemberNotFound);
        }

        let demoted = sqlx::query(
            "UPDATE team_members SET role = 'responder' WHERE team_id = $1 AND user_id = $2 AND role = 'manager'",
        )
        .bind(team_id)
        .bind(old_manager)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if demoted.rows_affected() != 1 {
            return Err(DomainError::NotManager);
        }

        let promoted = sqlx::query(
            "UPDATE team_members SET role = 'manager' WHERE team_id = $1 AND user_id = $2 AND role <> 'manager'",
        )
        .bind(team_id)
        .bind(new_manager)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if promoted.rows_affected() != 1 {
            return Err(DomainError::MemberNotFound);
        }

        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT team_id
            FROM team_members
            WHERE user_id = $1
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records.into_iter().map(|row| row.team_id).collect())
    }

    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT t.id, t.name, t.invitation_code, t.created_at, m.role
            FROM team_members m
            JOIN teams t ON t.id = m.team_id
            WHERE m.user_id = $1
            ORDER BY m.joined_at
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        records
            .into_iter()
            .map(|row| {
                Ok((
                    Team {
                        id: row.id,
                        name: row.name,
                        invitation_code: InvitationCode::from_existing(row.invitation_code),
                        created_at: row.created_at,
                    },
                    role_from_str(&row.role)?,
                ))
            })
            .collect()
    }

    async fn list_team_directory_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TeamDirectoryItem>, DomainError> {
        team_directory::list_for_user(&self.pool, user_id).await
    }

    async fn find_team_by_id(&self, team_id: Uuid) -> Result<Option<Team>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, name, invitation_code, created_at
            FROM teams
            WHERE id = $1
            "#,
            team_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.map(|row| Team {
            id: row.id,
            name: row.name,
            invitation_code: InvitationCode::from_existing(row.invitation_code),
            created_at: row.created_at,
        }))
    }

    async fn delete_team(&self, team_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM teams
            WHERE id = $1
            "#,
            team_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM team_members
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

    async fn kick_member_and_clear_assignments(
        &self,
        team_id: Uuid,
        requester_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        if lock_moderation_target(&mut tx, team_id, requester_id, target_user_id)
            .await?
            .is_none()
        {
            return Err(DomainError::MemberNotFound);
        }
        let removed = sqlx::query(
            "DELETE FROM team_members WHERE team_id = $1 AND user_id = $2 AND role <> 'manager'",
        )
        .bind(team_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if removed.rows_affected() != 1 {
            return Err(DomainError::MemberNotFound);
        }
        clear_member_assignments(&mut tx, team_id, target_user_id).await?;
        tx.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM team_members
            WHERE team_id = $1
            "#,
            team_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(record.count as u64)
    }

    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT u.id AS user_id, u.email, m.role, m.joined_at
            FROM team_members m
            JOIN users u ON u.id = m.user_id
            WHERE m.team_id = $1
            ORDER BY m.joined_at
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        records
            .into_iter()
            .map(|row| {
                Ok(TeamMemberView {
                    user_id: row.user_id,
                    email: row.email,
                    role: role_from_str(&row.role)?,
                    joined_at: row.joined_at,
                })
            })
            .collect()
    }

    async fn set_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE team_members SET role = $3
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            user_id,
            role.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError> {
        self.add_ban_impl(ban).await
    }

    async fn ban_member_and_clear_assignments(
        &self,
        ban: &TeamBan,
        requester_id: Uuid,
    ) -> Result<bool, DomainError> {
        self.ban_member_and_clear_assignments_impl(ban, requester_id).await
    }

    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError> {
        self.find_ban_impl(team_id, user_id).await
    }

    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
        self.list_bans_impl(team_id).await
    }

    async fn remove_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.remove_ban_impl(team_id, user_id).await
    }

    async fn save_team_image(&self, team_id: Uuid, image: &TeamImage) -> Result<(), DomainError> {
        self.save_team_image_impl(team_id, image).await
    }

    async fn find_team_image_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamImage>, DomainError> {
        self.find_team_image_for_member_impl(team_id, user_id).await
    }

    async fn delete_team_image(&self, team_id: Uuid) -> Result<(), DomainError> {
        self.delete_team_image_impl(team_id).await
    }
}

// --- TESTS (require a reachable Postgres; URL from the DATABASE_URL variable) ---

#[cfg(test)]
#[path = "team_tests.rs"]
mod tests;
