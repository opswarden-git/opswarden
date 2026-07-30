// --- server/src/adapters/pg/team.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::{
    BanKind, InvitationCode, Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamMemberView,
};
use crate::ports::TeamRepo;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "team_mapping.rs"]
mod team_mapping;

use team_mapping::{ban_kind, role_from_str, role_to_str};

pub struct PgTeamRepo {
    pool: PgPool,
}

impl PgTeamRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TeamRepo for PgTeamRepo {
    async fn save_team(&self, team: &Team) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO teams (id, name, invitation_code, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
            team.id,
            team.name,
            team.invitation_code.as_str(),
            team.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
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

        Ok(record.map(|row| role_from_str(&row.role)))
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
            role_to_str(role),
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
        // Single transaction, demote-then-promote: between the two statements
        // the team has zero Managers, so the `one_manager_per_team` index is
        // never violated and the swap is all-or-nothing.
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        sqlx::query!(
            r#"
            UPDATE team_members SET role = 'responder'
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            old_manager,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        sqlx::query!(
            r#"
            UPDATE team_members SET role = 'manager'
            WHERE team_id = $1 AND user_id = $2
            "#,
            team_id,
            new_manager,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

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

        Ok(records
            .into_iter()
            .map(|row| {
                (
                    Team {
                        id: row.id,
                        name: row.name,
                        invitation_code: InvitationCode::from_existing(row.invitation_code),
                        created_at: row.created_at,
                    },
                    role_from_str(&row.role),
                )
            })
            .collect())
    }

    async fn list_team_directory_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TeamDirectoryItem>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT
                t.id,
                t.name,
                t.invitation_code,
                t.created_at,
                membership.role,
                (SELECT COUNT(*) FROM team_members members WHERE members.team_id = t.id) AS "member_count!",
                (SELECT COUNT(*) FROM incidents incidents
                    WHERE incidents.team_id = t.id AND incidents.status <> 'resolved') AS "active_incident_count!",
                (SELECT COUNT(*) FROM releases releases
                    WHERE releases.team_id = t.id
                      AND releases.base_state IN ('created', 'in_progress')) AS "active_release_count!",
                (SELECT COUNT(*) FROM releases releases
                    WHERE releases.team_id = t.id
                      AND releases.base_state = 'in_progress'
                      AND EXISTS (
                          SELECT 1
                          FROM release_incidents links
                          JOIN incidents incidents ON incidents.id = links.incident_id
                          WHERE links.release_id = releases.id
                            AND incidents.status <> 'resolved'
                      )) AS "blocked_release_count!"
            FROM team_members membership
            JOIN teams t ON t.id = membership.team_id
            WHERE membership.user_id = $1
            ORDER BY membership.joined_at
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records
            .into_iter()
            .map(|row| TeamDirectoryItem {
                team: Team {
                    id: row.id,
                    name: row.name,
                    invitation_code: InvitationCode::from_existing(row.invitation_code),
                    created_at: row.created_at,
                },
                role: role_from_str(&row.role),
                member_count: row.member_count as u64,
                active_incident_count: row.active_incident_count as u64,
                active_release_count: row.active_release_count as u64,
                blocked_release_count: row.blocked_release_count as u64,
            })
            .collect())
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

        Ok(records
            .into_iter()
            .map(|row| TeamMemberView {
                user_id: row.user_id,
                email: row.email,
                role: role_from_str(&row.role),
                joined_at: row.joined_at,
            })
            .collect())
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
            role_to_str(role),
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError> {
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

    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError> {
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

    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
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

    async fn remove_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
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
}

// --- TESTS (require a reachable Postgres; URL from the DATABASE_URL variable) ---

#[cfg(test)]
#[path = "team_tests.rs"]
mod tests;
