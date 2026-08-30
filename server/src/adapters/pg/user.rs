// --- server/src/adapters/pg/user.rs ---

use crate::domain::error::DomainError;
use crate::domain::user::{Email, Locale, User};
use crate::ports::UserRepo;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    locale: String,
    created_at: DateTime<Utc>,
}

fn user_from_row(row: UserRow) -> Result<User, DomainError> {
    Ok(User {
        id: row.id,
        email: Email::new(row.email)?,
        password_hash: row.password_hash,
        locale: Locale::try_from(row.locale.as_str())?,
        created_at: row.created_at,
    })
}

pub struct PgUserRepo {
    pool: PgPool,
}

impl PgUserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepo for PgUserRepo {
    async fn find_by_id(&self, user_id: uuid::Uuid) -> Result<Option<User>, DomainError> {
        let record = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, password_hash, locale, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        record.map(user_from_row).transpose()
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let record = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, password_hash, locale, created_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        record.map(user_from_row).transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, locale, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user.id)
        .bind(user.email.as_str())
        .bind(&user.password_hash)
        .bind(user.locale.as_str())
        .bind(user.created_at)
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db) if db.is_unique_violation() => DomainError::UserAlreadyExists,
            _ => DomainError::Storage,
        })?;

        Ok(())
    }

    async fn update_locale(&self, user_id: Uuid, locale: Locale) -> Result<(), DomainError> {
        let updated = sqlx::query("UPDATE users SET locale = $1 WHERE id = $2")
            .bind(locale.as_str())
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|_| DomainError::Storage)?;
        if updated.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }
        Ok(())
    }

    async fn delete_account(&self, user_id: uuid::Uuid) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        // Locking the account prevents a concurrent team creation from adding a
        // new Manager membership through its FK while this decision is made.
        let exists = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        if exists.is_none() {
            return Err(DomainError::InvalidToken);
        }

        // Lock every managed team in stable order. A membership INSERT takes a
        // key-share lock on its parent Team, so no member can join between the
        // count and deletion of a single-member team.
        let managed_team_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT team.id
            FROM teams team
            JOIN team_members manager
              ON manager.team_id = team.id
             AND manager.user_id = $1
             AND manager.role = 'manager'
            ORDER BY team.id
            FOR UPDATE OF team, manager
            "#,
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        for team_id in &managed_team_ids {
            let member_count = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM team_members WHERE team_id = $1",
            )
            .bind(team_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
            if member_count > 1 {
                return Err(DomainError::MustTransferManagerFirst);
            }
        }
        if !managed_team_ids.is_empty() {
            sqlx::query("DELETE FROM teams WHERE id = ANY($1)")
                .bind(&managed_team_ids)
                .execute(&mut *tx)
                .await
                .map_err(|_| DomainError::Storage)?;
        }

        // Plain memberships cascade. Incident assignments, durable events and
        // timeline authors are pseudonymized by their foreign keys.

        let deleted = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        if deleted.rows_affected() != 1 {
            return Err(DomainError::InvalidToken);
        }
        tx.commit().await.map_err(|_| DomainError::Storage)
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::incident::PgIncidentRepo;
    use crate::adapters::pg::team::PgTeamRepo;
    use crate::adapters::pg::timeline::PgTimelineRepo;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::{Role, Team};
    use crate::domain::timeline::TimelineEntry;
    use crate::ports::{IncidentRepo, TeamRepo, TimelineRepo};
    #[sqlx::test]
    async fn it_saves_and_finds_a_user_in_postgres(pool: PgPool) {
        let repo = PgUserRepo::new(pool);

        let email_str = format!("integration_{}@opswarden.com", uuid::Uuid::new_v4());
        let email = Email::new(email_str).unwrap();
        let user = User::new(email.clone(), "my_super_hash");

        let save_result = repo.save(&user).await;
        assert!(save_result.is_ok());

        let found = repo.find_by_email(email.as_str()).await.unwrap();

        assert!(found.is_some());
        let found_user = found.unwrap();
        assert_eq!(found_user.id, user.id);
        assert_eq!(found_user.email.as_str(), user.email.as_str());
        assert_eq!(found_user.password_hash, "my_super_hash");
        assert_eq!(found_user.locale, Locale::En);

        repo.update_locale(user.id, Locale::Fr).await.unwrap();
        assert_eq!(
            repo.find_by_id(user.id).await.unwrap().unwrap().locale,
            Locale::Fr
        );
    }

    #[sqlx::test]
    async fn one_shared_team_blocks_deletion_without_removing_lone_teams(pool: PgPool) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let manager = User::new(
            Email::new(format!("manager_{}@opswarden.com", Uuid::new_v4())).unwrap(),
            "hash",
        );
        let member = User::new(
            Email::new(format!("member_{}@opswarden.com", Uuid::new_v4())).unwrap(),
            "hash",
        );
        users.save(&manager).await.unwrap();
        users.save(&member).await.unwrap();
        let lone_team = Team::new("Lone managed team").unwrap();
        let shared_team = Team::new("Shared managed team").unwrap();
        teams
            .create_team_with_manager(&lone_team, manager.id)
            .await
            .unwrap();
        teams
            .create_team_with_manager(&shared_team, manager.id)
            .await
            .unwrap();
        teams
            .add_member(shared_team.id, member.id, Role::Observer)
            .await
            .unwrap();

        assert_eq!(
            users.delete_account(manager.id).await.unwrap_err(),
            DomainError::MustTransferManagerFirst
        );

        assert!(users.find_by_id(manager.id).await.unwrap().is_some());
        assert!(teams.find_team_by_id(lone_team.id).await.unwrap().is_some());
        assert!(teams
            .find_team_by_id(shared_team.id)
            .await
            .unwrap()
            .is_some());
    }

    #[sqlx::test]
    async fn failed_account_delete_restores_the_owned_team(pool: PgPool) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let manager = User::new(
            Email::new(format!("rollback_{}@opswarden.com", Uuid::new_v4())).unwrap(),
            "hash",
        );
        users.save(&manager).await.unwrap();
        let team = Team::new("Rollback managed team").unwrap();
        teams
            .create_team_with_manager(&team, manager.id)
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE FUNCTION reject_account_delete() RETURNS trigger AS $$
            BEGIN
                RAISE EXCEPTION 'injected account deletion failure';
            END;
            $$ LANGUAGE plpgsql
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            CREATE TRIGGER reject_account_delete
            BEFORE DELETE ON users
            FOR EACH ROW EXECUTE FUNCTION reject_account_delete()
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            users.delete_account(manager.id).await.unwrap_err(),
            DomainError::Storage
        );

        assert!(users.find_by_id(manager.id).await.unwrap().is_some());
        assert!(teams.find_team_by_id(team.id).await.unwrap().is_some());
        assert_eq!(
            teams.find_member_role(team.id, manager.id).await.unwrap(),
            Some(Role::Manager)
        );
    }

    #[sqlx::test]
    async fn lone_manager_deletes_owned_teams_and_account(pool: PgPool) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let manager = User::new(
            Email::new(format!("lone_{}@opswarden.com", Uuid::new_v4())).unwrap(),
            "hash",
        );
        users.save(&manager).await.unwrap();
        let team = Team::new("Disposable managed team").unwrap();
        teams
            .create_team_with_manager(&team, manager.id)
            .await
            .unwrap();

        users.delete_account(manager.id).await.unwrap();

        assert!(users.find_by_id(manager.id).await.unwrap().is_none());
        assert!(teams.find_team_by_id(team.id).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn delete_account_pseudonymizes_notes_and_keeps_operational_history(pool: PgPool) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let incidents = PgIncidentRepo::new(pool.clone());
        let timeline = PgTimelineRepo::new(pool);

        let email = Email::new(format!("delete_{}@opswarden.com", uuid::Uuid::new_v4())).unwrap();
        let user = User::new(email.clone(), "hash_to_delete");
        users.save(&user).await.unwrap();

        // Deleting a plain member removes the user and membership (FK cascade),
        // but never the team, incident, or operational note.
        let manager_email =
            Email::new(format!("manager_{}@opswarden.com", uuid::Uuid::new_v4())).unwrap();
        let manager = User::new(manager_email, "hash");
        users.save(&manager).await.unwrap();
        let team = Team::new(format!("Delete {}", uuid::Uuid::new_v4())).unwrap();
        teams
            .create_team_with_manager(&team, manager.id)
            .await
            .unwrap();
        teams
            .add_member(team.id, user.id, Role::Observer)
            .await
            .unwrap();

        let incident = Incident::new(team.id, "delete account cascade", Severity::High).unwrap();
        incidents.save_incident(&incident).await.unwrap();
        let entry = TimelineEntry::new(incident.id, user.id, "owned by deleted user").unwrap();
        timeline.append_entry(&entry).await.unwrap();

        users.delete_account(user.id).await.unwrap();

        assert!(users.find_by_email(email.as_str()).await.unwrap().is_none());
        // Membership gone (FK cascade)...
        assert!(teams
            .list_team_ids_for_user(user.id)
            .await
            .unwrap()
            .is_empty());
        // ...but the team and its incident survive (no collateral destruction).
        assert!(teams
            .find_by_invitation_code(team.invitation_code.as_str())
            .await
            .unwrap()
            .is_some());
        assert!(incidents
            .find_incident_by_id(incident.id)
            .await
            .unwrap()
            .is_some());
        let notes = timeline
            .list_entries_for_incident(incident.id, None, 10)
            .await
            .unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].content, "owned by deleted user");
        assert_eq!(notes[0].author_id, None);
    }
}
