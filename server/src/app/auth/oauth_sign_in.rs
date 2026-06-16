use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::user::{Email, User};
use crate::ports::{PasswordHasher, TokenService, UserRepo};

pub struct OAuthSignInCommand {
    pub email: String,
}

pub struct OAuthSignInResult {
    pub token: String,
}

pub struct OAuthSignInUseCase {
    users: Arc<dyn UserRepo + Send + Sync>,
    hasher: Arc<dyn PasswordHasher + Send + Sync>,
    tokens: Arc<dyn TokenService + Send + Sync>,
}

impl OAuthSignInUseCase {
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

    pub async fn sign_in(&self, cmd: OAuthSignInCommand) -> Result<OAuthSignInResult, DomainError> {
        let email = Email::new(cmd.email)?;

        let user = match self.users.find_by_email(email.as_str()).await? {
            Some(user) => user,
            None => {
                let password_hash = self
                    .hasher
                    .hash(&format!("google-oauth-{}", Uuid::new_v4()))?;
                let user = User::new(email, password_hash);
                self.users.save(&user).await?;
                user
            }
        };

        let token = self.tokens.generate_token(user.id)?;
        Ok(OAuthSignInResult { token })
    }
}
