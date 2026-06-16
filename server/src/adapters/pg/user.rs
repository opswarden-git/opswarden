// --- server/src/adapters/pg/user.rs ---

use crate::domain::error::DomainError;
use crate::domain::user::{Email, User};
use crate::ports::UserRepo;
use async_trait::async_trait;
use sqlx::PgPool;

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
        let record = sqlx::query!(
            r#"
            SELECT id, email, password_hash, created_at
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        record
            .map(|row| {
                let email = Email::new(row.email)?;
                Ok(User {
                    id: row.id,
                    email,
                    password_hash: row.password_hash,
                    created_at: row.created_at,
                })
            })
            .transpose()
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let record = sqlx::query!(
            r#"
            SELECT id, email, password_hash, created_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        match record {
            Some(row) => {
                let e = Email::new(row.email)?;
                Ok(Some(User {
                    id: row.id,
                    email: e,
                    password_hash: row.password_hash,
                    created_at: row.created_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
            user.id,
            user.email.as_str(),
            user.password_hash,
            user.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db) if db.is_unique_violation() => DomainError::UserAlreadyExists,
            _ => DomainError::Storage,
        })?;

        Ok(())
    }

    async fn delete_account(&self, user_id: uuid::Uuid) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        // Teams are deliberately NOT deleted here: the use-case refuses account
        // deletion while the user still manages a team, so we never orphan a team
        // or destroy other members' data. Memberships cascade and incident
        // assignments are set null by the FKs; only the user's timeline entries
        // (FK is ON DELETE RESTRICT) must be removed explicitly, first.
        sqlx::query!(
            r#"
            DELETE FROM timeline_entries
            WHERE author_id = $1
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        let deleted = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        tx.commit().await.map_err(|_| DomainError::Storage)?;

        if deleted.rows_affected() == 0 {
            return Err(DomainError::InvalidToken);
        }

        Ok(())
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
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn it_saves_and_finds_a_user_in_postgres() {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string()
        });
        let pool = PgPoolOptions::new().connect(&database_url).await.unwrap();
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
    }

    #[tokio::test]
    async fn delete_account_removes_user_and_timeline_but_keeps_the_team_in_postgres() {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://opswarden:opswarden@localhost:5433/opswarden".to_string()
        });
        let pool = PgPoolOptions::new().connect(&database_url).await.unwrap();
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let incidents = PgIncidentRepo::new(pool.clone());
        let timeline = PgTimelineRepo::new(pool);

        let email = Email::new(format!("delete_{}@opswarden.com", uuid::Uuid::new_v4())).unwrap();
        let user = User::new(email.clone(), "hash_to_delete");
        users.save(&user).await.unwrap();

        // A plain member (Manager-gating lives in the use-case): deleting the
        // account removes the user, their timeline entries and their membership
        // (FK cascade) — but never the team or its incidents.
        let team = Team::new(format!("Delete {}", uuid::Uuid::new_v4())).unwrap();
        teams.save_team(&team).await.unwrap();
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
    }
}
