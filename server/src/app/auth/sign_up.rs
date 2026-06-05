// --- server/src/app/auth/sign_up.rs ---
use crate::domain::error::DomainError;
use crate::domain::user::{Email, User};
use crate::ports::{PasswordHasher, UserRepo};
use std::sync::Arc;

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
        Self { users, hasher }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::tests::{MockHasher, MockUserRepo};

    #[tokio::test]
    async fn sign_up_success_when_user_does_not_exist() {
        let repo = Arc::new(MockUserRepo {
            simulate_user_exists: false,
        });
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
        let repo = Arc::new(MockUserRepo {
            simulate_user_exists: true,
        });
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
