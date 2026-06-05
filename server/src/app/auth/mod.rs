// --- server/src/app/auth/mod.rs ---
pub mod sign_up;
pub mod sign_in;

pub use sign_up::{SignUpCommand, SignUpResult, SignUpUseCase};
pub use sign_in::{SignInCommand, SignInResult, SignInUseCase};

// Shared mocks for tests in this module
#[cfg(test)]
pub(crate) mod tests {
    use crate::domain::error::DomainError;
    use crate::domain::user::{Email, User};
    use crate::ports::{PasswordHasher, TokenService, UserRepo};
    use async_trait::async_trait;

    pub struct MockUserRepo {
        pub simulate_user_exists: bool,
    }

    #[async_trait]
    impl UserRepo for MockUserRepo {
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
        fn verify_token(&self, token: &str) -> Result<uuid::Uuid, DomainError> {
            if token == "mock_jwt_token" {
                Ok(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
            } else {
                Err(DomainError::InvalidToken)
            }
        }
    }
}
