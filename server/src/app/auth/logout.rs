use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::domain::error::DomainError;
use crate::ports::TokenRevocationRepo;

pub struct LogoutCommand {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

pub struct LogoutUseCase {
    revoked_tokens: Arc<dyn TokenRevocationRepo + Send + Sync>,
}

impl LogoutUseCase {
    pub fn new(revoked_tokens: Arc<dyn TokenRevocationRepo + Send + Sync>) -> Self {
        Self { revoked_tokens }
    }

    /// Revoking the same bearer token more than once is a no-op. The storage
    /// layer keeps the operation idempotent so clients can safely retry logout.
    pub async fn logout(&self, cmd: LogoutCommand) -> Result<(), DomainError> {
        self.revoked_tokens.revoke(&cmd.token, cmd.expires_at).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::tests::MockTokenRevocationRepo;

    #[tokio::test]
    async fn logout_revokes_the_presented_token() {
        let repo = Arc::new(MockTokenRevocationRepo::default());
        let use_case = LogoutUseCase::new(repo.clone());
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        use_case
            .logout(LogoutCommand {
                token: "mock_jwt_token".to_string(),
                expires_at,
            })
            .await
            .unwrap();

        let revoked = repo.revoked.lock().unwrap();
        assert_eq!(
            revoked.as_slice(),
            &[("mock_jwt_token".to_string(), expires_at)]
        );
    }
}
