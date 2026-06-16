// --- server/src/app/auth/mod.rs ---
pub mod delete_account;
pub mod logout;
pub mod oauth_sign_in;
pub mod sign_in;
pub mod sign_up;

pub use delete_account::{DeleteAccountCommand, DeleteAccountUseCase};
pub use logout::{LogoutCommand, LogoutUseCase};
pub use oauth_sign_in::{OAuthSignInCommand, OAuthSignInUseCase};
pub use sign_in::{SignInCommand, SignInResult, SignInUseCase};
pub use sign_up::{SignUpCommand, SignUpResult, SignUpUseCase};

// Shared mocks for tests in this module
#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::error::DomainError;
    use crate::domain::user::{Email, User};
    use crate::ports::{PasswordHasher, TokenClaims, TokenRevocationRepo, TokenService, UserRepo};
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    pub struct MockUserRepo {
        pub simulate_user_exists: bool,
    }

    #[async_trait]
    impl UserRepo for MockUserRepo {
        async fn find_by_id(&self, _user_id: uuid::Uuid) -> Result<Option<User>, DomainError> {
            if self.simulate_user_exists {
                let email = Email::new("existing@opswarden.com").unwrap();
                Ok(Some(User::new(email, "old_hash")))
            } else {
                Ok(None)
            }
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
            if self.simulate_user_exists {
                let email = Email::new("existing@opswarden.com").unwrap();
                Ok(Some(User::new(email, "old_hash")))
            } else {
                Ok(None)
            }
        }
        async fn save(&self, _user: &User) -> Result<(), DomainError> {
            Ok(())
        }

        async fn delete_account(&self, _user_id: uuid::Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    pub struct MockHasher;
    impl PasswordHasher for MockHasher {
        fn hash(&self, _password: &str) -> Result<String, DomainError> {
            Ok("hashed_password".to_string())
        }
        fn verify(&self, password: &str, _hash: &str) -> Result<bool, DomainError> {
            Ok(password == "correct_password")
        }
    }

    pub struct MockTokenService;
    impl TokenService for MockTokenService {
        fn generate_token(&self, _user_id: uuid::Uuid) -> Result<String, DomainError> {
            Ok("mock_jwt_token".to_string())
        }
        fn verify_token(&self, token: &str) -> Result<TokenClaims, DomainError> {
            if token == "mock_jwt_token" {
                Ok(TokenClaims {
                    user_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                })
            } else {
                Err(DomainError::InvalidToken)
            }
        }
    }

    #[derive(Default)]
    pub struct MockTokenRevocationRepo {
        pub revoked: Mutex<Vec<(String, DateTime<Utc>)>>,
    }

    #[async_trait]
    impl TokenRevocationRepo for MockTokenRevocationRepo {
        async fn revoke(&self, token: &str, expires_at: DateTime<Utc>) -> Result<(), DomainError> {
            self.revoked
                .lock()
                .unwrap()
                .push((token.to_string(), expires_at));
            Ok(())
        }

        async fn is_revoked(&self, token: &str) -> Result<bool, DomainError> {
            Ok(self
                .revoked
                .lock()
                .unwrap()
                .iter()
                .any(|(revoked, _)| revoked == token))
        }
    }
}
