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
        .map_err(|_| DomainError::UserAlreadyExists)?;

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
        .map_err(|_| DomainError::UserAlreadyExists)?;

        Ok(())
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;
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
}
