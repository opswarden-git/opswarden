// --- server/src/app/auth/sign_in.rs ---
use crate::domain::error::DomainError;
use crate::domain::user::Email;
use crate::ports::{PasswordHasher, TokenService, UserRepo};
use std::sync::Arc;

pub struct SignInCommand {
    pub email: String,
    pub plain_password: String,
}

#[derive(Debug, PartialEq)]
pub struct SignInResult {
    pub token: String,
}

pub struct SignInUseCase {
    users: Arc<dyn UserRepo + Send + Sync>,
    hasher: Arc<dyn PasswordHasher + Send + Sync>,
    tokens: Arc<dyn TokenService + Send + Sync>,
}

impl SignInUseCase {
    pub fn new(
        users: Arc<dyn UserRepo + Send + Sync>,
        hasher: Arc<dyn PasswordHasher + Send + Sync>,
        tokens: Arc<dyn TokenService + Send + Sync>,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
        }
    }

    pub async fn sign_in(&self, cmd: SignInCommand) -> Result<SignInResult, DomainError> {
        let email = Email::new(cmd.email)?;
        let user = self.users.find_by_email(email.as_str()).await?;

        let user = match user {
            Some(u) => u,
            None => return Err(DomainError::InvalidCredentials),
        };

        let is_valid = self
            .hasher
            .verify(&cmd.plain_password, &user.password_hash)?;
        if !is_valid {
            return Err(DomainError::InvalidCredentials);
        }

        let token = self.tokens.generate_token(user.id)?;
        Ok(SignInResult { token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::tests::{MockHasher, MockTokenService, MockUserRepo};

    #[tokio::test]
    async fn sign_in_success_with_valid_credentials() {
        let repo = Arc::new(MockUserRepo {
            simulate_user_exists: true,
        });
        let hasher = Arc::new(MockHasher);
        let tokens = Arc::new(MockTokenService);
        let use_case = SignInUseCase::new(repo, hasher, tokens);

        let cmd = SignInCommand {
            email: "existing@opswarden.com".to_string(),
            plain_password: "correct_password".to_string(),
        };

        let result = use_case.sign_in(cmd).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().token, "mock_jwt_token");
    }

    #[tokio::test]
    async fn sign_in_fails_with_invalid_password() {
        let repo = Arc::new(MockUserRepo {
            simulate_user_exists: true,
        });
        let hasher = Arc::new(MockHasher);
        let tokens = Arc::new(MockTokenService);
        let use_case = SignInUseCase::new(repo, hasher, tokens);

        let cmd = SignInCommand {
            email: "existing@opswarden.com".to_string(),
            plain_password: "wrong_password".to_string(),
        };

        let result = use_case.sign_in(cmd).await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidCredentials);
    }

    #[tokio::test]
    async fn sign_in_fails_if_user_does_not_exist() {
        let repo = Arc::new(MockUserRepo {
            simulate_user_exists: false,
        });
        let hasher = Arc::new(MockHasher);
        let tokens = Arc::new(MockTokenService);
        let use_case = SignInUseCase::new(repo, hasher, tokens);

        let cmd = SignInCommand {
            email: "unknown@opswarden.com".to_string(),
            plain_password: "correct_password".to_string(),
        };

        let result = use_case.sign_in(cmd).await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidCredentials);
    }
}
