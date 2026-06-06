// --- server/src/adapters/pg/team.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::{InvitationCode, Role, Team};
use crate::ports::TeamRepo;
use async_trait::async_trait;
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
}

// --- TESTS (require a reachable Postgres; URL from the DATABASE_URL variable) ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::user::{Email, User};
    use crate::ports::UserRepo;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string()
        });
        PgPoolOptions::new().connect(&database_url).await.unwrap()
    }

    /// Persist a throwaway user so membership FKs resolve.
    async fn seed_user(pool: &PgPool) -> Uuid {
        let users = PgUserRepo::new(pool.clone());
        let email = Email::new(format!("team_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();
        user.id
    }

    #[tokio::test]
    async fn it_creates_joins_and_transfers_in_postgres() {
        let pool = test_pool().await;
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

    #[tokio::test]
    async fn joining_twice_is_rejected_by_the_database() {
        let pool = test_pool().await;
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

    #[tokio::test]
    async fn unknown_invitation_code_returns_none() {
        let repo = PgTeamRepo::new(test_pool().await);

        let found = repo.find_by_invitation_code("OPS-NOPE99").await.unwrap();

        assert!(found.is_none());
    }
}
