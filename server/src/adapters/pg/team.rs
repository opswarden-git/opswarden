// --- server/src/adapters/pg/team.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::{BanKind, InvitationCode, Role, Team, TeamBan, TeamMemberView};
use crate::ports::TeamRepo;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgTeamRepo {
    pool: PgPool,
}

impl PgTeamRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `Role` as stored in the `team_members.role` text column (kept out of the
/// domain so `Role` stays free of persistence concerns).
fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Observer => "observer",
        Role::Responder => "responder",
        Role::Manager => "manager",
    }
}

/// Inverse of `role_to_str`. The DB `check` constraint guarantees a valid value;
/// anything unexpected falls back to the least-privileged role by design.
fn role_from_str(value: &str) -> Role {
    match value {
        "manager" => Role::Manager,
        "responder" => Role::Responder,
        _ => Role::Observer,
    }
}

/// Map the nullable `team_bans.expires_at` column to a `BanKind`
/// (NULL = permanent, a timestamp = temporary).
fn ban_kind(expires_at: Option<DateTime<Utc>>) -> BanKind {
    match expires_at {
        Some(expires_at) => BanKind::Temporary { expires_at },
        None => BanKind::Permanent,
    }
}

#[async_trait]
impl TeamRepo for PgTeamRepo {
    async fn save_team(&self, team: &Team) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO teams (id, name, invitation_code)
            VALUES ($1, $2, $3)
            "#,
            team.id,
            team.name,
            team.invitation_code.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, name, invitation_code
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
            SELECT t.id, t.name, t.invitation_code, m.role
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
                    },
                    role_from_str(&row.role),
                )
            })
            .collect())
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
            SELECT u.id AS user_id, u.email, m.role
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

    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBan>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT user_id, expires_at, reason, created_by, created_at
            FROM team_bans
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(rows
            .into_iter()
            .map(|r| TeamBan {
                team_id,
                user_id: r.user_id,
                kind: ban_kind(r.expires_at),
                reason: r.reason,
                created_by: r.created_by,
                created_at: r.created_at,
            })
            .collect())
    }
}

// --- TESTS (require a reachable Postgres; URL from the DATABASE_URL variable) ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::user::{Email, User};
    use crate::ports::UserRepo;
    /// Persist a throwaway user so membership FKs resolve.
    async fn seed_user(pool: &PgPool) -> Uuid {
        let users = PgUserRepo::new(pool.clone());
        let email = Email::new(format!("team_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();
        user.id
    }

    #[sqlx::test]
    async fn it_creates_joins_and_transfers_in_postgres(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());

        let manager = seed_user(&pool).await;
        let newcomer = seed_user(&pool).await;

        let team = Team::new("Postgres Crew").unwrap();
        repo.save_team(&team).await.unwrap();

        // Resolve by invitation code (the join entry point).
        let found = repo
            .find_by_invitation_code(team.invitation_code.as_str())
            .await
            .unwrap();
        assert_eq!(found.unwrap().id, team.id);

        // Creator is Manager, newcomer joins as Observer.
        repo.add_member(team.id, manager, Role::Manager)
            .await
            .unwrap();
        repo.add_member(team.id, newcomer, Role::Observer)
            .await
            .unwrap();
        assert_eq!(
            repo.find_member_role(team.id, manager).await.unwrap(),
            Some(Role::Manager)
        );

        // Atomic hand-over upholds the single-Manager invariant.
        repo.transfer_manager(team.id, manager, newcomer)
            .await
            .unwrap();
        assert_eq!(
            repo.find_member_role(team.id, manager).await.unwrap(),
            Some(Role::Responder)
        );
        assert_eq!(
            repo.find_member_role(team.id, newcomer).await.unwrap(),
            Some(Role::Manager)
        );
    }

    #[sqlx::test]
    async fn joining_twice_is_rejected_by_the_database(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());

        let user = seed_user(&pool).await;
        let team = Team::new("Dup Guard").unwrap();
        repo.save_team(&team).await.unwrap();

        repo.add_member(team.id, user, Role::Observer)
            .await
            .unwrap();
        let again = repo.add_member(team.id, user, Role::Observer).await;

        assert_eq!(again.unwrap_err(), DomainError::AlreadyMember);
    }

    #[sqlx::test]
    async fn unknown_invitation_code_returns_none(pool: PgPool) {
        let repo = PgTeamRepo::new(pool);

        let found = repo.find_by_invitation_code("OPS-NOPE99").await.unwrap();

        assert!(found.is_none());
    }

    #[sqlx::test]
    async fn it_lists_members_with_email_and_role(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());

        let manager = seed_user(&pool).await;
        let observer = seed_user(&pool).await;
        let team = Team::new("Roster Crew").unwrap();
        repo.save_team(&team).await.unwrap();
        repo.add_member(team.id, manager, Role::Manager)
            .await
            .unwrap();
        repo.add_member(team.id, observer, Role::Observer)
            .await
            .unwrap();

        let members = repo.list_members(team.id).await.unwrap();

        assert_eq!(members.len(), 2);
        let manager_row = members.iter().find(|m| m.user_id == manager).unwrap();
        assert_eq!(manager_row.role, Role::Manager);
        assert!(manager_row.email.contains('@'));
        assert!(members
            .iter()
            .any(|m| m.user_id == observer && m.role == Role::Observer));
    }

    #[sqlx::test]
    async fn it_lists_no_members_for_an_unknown_team(pool: PgPool) {
        let repo = PgTeamRepo::new(pool);

        let members = repo.list_members(Uuid::new_v4()).await.unwrap();

        assert!(members.is_empty());
    }

    #[sqlx::test]
    async fn it_sets_a_member_role_in_postgres(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());

        let member = seed_user(&pool).await;
        let team = Team::new("Role Crew").unwrap();
        repo.save_team(&team).await.unwrap();
        repo.add_member(team.id, member, Role::Observer)
            .await
            .unwrap();

        repo.set_member_role(team.id, member, Role::Responder)
            .await
            .unwrap();

        assert_eq!(
            repo.find_member_role(team.id, member).await.unwrap(),
            Some(Role::Responder)
        );
    }

    #[sqlx::test]
    async fn it_stores_finds_and_upserts_bans_in_postgres(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());
        let manager = seed_user(&pool).await;
        let target = seed_user(&pool).await;
        let team = Team::new("Ban Crew").unwrap();
        repo.save_team(&team).await.unwrap();

        // No ban initially.
        assert!(repo.find_ban(team.id, target).await.unwrap().is_none());

        // Permanent ban with a reason.
        let ban = TeamBan::permanent(team.id, target, manager, Some("spam".to_string()));
        repo.add_ban(&ban).await.unwrap();

        let found = repo.find_ban(team.id, target).await.unwrap().unwrap();
        assert!(matches!(found.kind, BanKind::Permanent));
        assert!(found.is_active(Utc::now()));
        assert_eq!(found.reason.as_deref(), Some("spam"));
        assert_eq!(found.created_by, Some(manager));

        // Re-banning the same user upserts (one row, now temporary).
        let expires = Utc::now() + chrono::Duration::hours(1);
        let temp = TeamBan::temporary(team.id, target, manager, expires, None).unwrap();
        repo.add_ban(&temp).await.unwrap();

        let bans = repo.list_bans(team.id).await.unwrap();
        assert_eq!(bans.len(), 1);
        assert!(matches!(bans[0].kind, BanKind::Temporary { .. }));
        assert!(bans[0].is_active(Utc::now()));
        assert!(bans[0].reason.is_none());
    }

    #[sqlx::test]
    async fn deleting_the_moderator_account_keeps_the_ban_and_nulls_created_by(pool: PgPool) {
        let repo = PgTeamRepo::new(pool.clone());
        let users = PgUserRepo::new(pool.clone());
        let moderator = seed_user(&pool).await;
        let target = seed_user(&pool).await;
        let team = Team::new("Ban Crew").unwrap();
        repo.save_team(&team).await.unwrap();

        repo.add_ban(&TeamBan::permanent(team.id, target, moderator, None))
            .await
            .unwrap();

        // The moderator deletes their account: the FK is ON DELETE SET NULL, so
        // this must not fail and the ban must survive.
        users.delete_account(moderator).await.unwrap();

        let ban = repo.find_ban(team.id, target).await.unwrap().unwrap();
        assert!(ban.is_active(Utc::now()));
        assert_eq!(ban.created_by, None);
    }
}
