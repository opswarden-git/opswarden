// --- server/src/app/auth.rs ---

use std::sync::Arc;

use crate::domain::user::{Email, User};
use crate::domain::error::DomainError;
use crate::ports::{PasswordHasher, UserRepo};
pub struct SignUpCommand {
    pub email: String,
    pub plain_password: String,
}

#[derive(Debug, PartialEq)]
pub struct SignUpResult {
    pub email: String,
    pub plain_password: String,
}

pub struct SignUpUseCase {
    users: Arc<dyn UserRepo + Send + Sync>,
    hasher: Arc<dyn PasswordHasher + Send + Sync>,
}

impl SignUpUseCase {
    pub fn new(
        users: Arc<dyn UserRepo + Send + Sync>,
        hasher: Arc<dyn PasswordHasher + Send + Sync>,
    ) -> Self {
        Self {
            users,
            hasher,
        }
    }

    pub async fn sign_up(&self, cmd: SignUpCommand) -> Result<SignUpResult, DomainError> {
        let email = Email::new(cmd.email)?;
        let user = self.users.find_by_email(email.as_str()).await?;
        if user.is_some() {
            return Err(DomainError::UserAlreadyExists);
        }
        let password_hash = self.hasher.hash(&cmd.plain_password)?;
        let user = User::new(email, password_hash);
        self.users.save(&user).await?;
        Ok(SignUpResult {
            email: user.email.as_str().to_string(),
            plain_password: cmd.plain_password,
        })
    }
}

// TEST
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // --- MOCKS ---
    struct MockUserRepo {
        pub simulate_user_exists: bool,
    }

    #[async_trait]
    impl UserRepo for MockUserRepo {
        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
            if self.simulate_user_exists {
                // On fait croire que quelqu'un est déjà en base
                let email = Email::new("existing@opswarden.com").unwrap();
                Ok(Some(User::new(email, "old_hash")))
            } else {
                Ok(None)
            }
        }
        async fn save(&self, _user: &User) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct MockHasher;
    impl PasswordHasher for MockHasher {
        fn hash(&self, password: &str) -> Result<String, DomainError> {
            Ok(format!("hashed_{}", password))
        }
    }

    // --- TESTS ---
    #[tokio::test]
    async fn sign_up_success_when_user_does_not_exist() {
        let repo = Arc::new(MockUserRepo { simulate_user_exists: false });
        let hasher = Arc::new(MockHasher);
        let use_case = SignUpUseCase::new(repo, hasher);

        let cmd = SignUpCommand {
            email: "new@opswarden.com".to_string(),
            plain_password: "password123".to_string(),
        };

        let result = use_case.sign_up(cmd).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "new@opswarden.com");
    }

    #[tokio::test]
    async fn sign_up_fails_if_user_already_exists() {
        let repo = Arc::new(MockUserRepo { simulate_user_exists: true });
        let hasher = Arc::new(MockHasher);
        let use_case = SignUpUseCase::new(repo, hasher);

        let cmd = SignUpCommand {
            email: "existing@opswarden.com".to_string(),
            plain_password: "password123".to_string(),
        };

        let result = use_case.sign_up(cmd).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::UserAlreadyExists);
    }
}
