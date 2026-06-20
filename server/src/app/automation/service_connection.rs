// --- server/src/app/automation/service_connection.rs ---
//
// Connecting a third-party service from the app: persist its secret in the vault
// and report whether it is configured. The connection is **global to the server**
// — the vault is keyed by service name, not by team or user — so it mirrors the
// boot-time `GITHUB_WEBHOOK_SECRET`, and any authenticated user can (re)configure
// it. A per-team connection would need a team-scoped vault key (future work).

use std::sync::Arc;

use crate::domain::error::DomainError;
use crate::ports::SecretVault;

/// Services connectable from the app; the vault is keyed by this name. Today only
/// GitHub (its inbound-webhook HMAC secret).
pub const GITHUB_SERVICE: &str = "github";

pub struct ServiceConnectionUseCase {
    vault: Arc<dyn SecretVault>,
}

impl ServiceConnectionUseCase {
    pub fn new(vault: Arc<dyn SecretVault>) -> Self {
        Self { vault }
    }

    /// Encrypt and store the GitHub inbound-webhook HMAC secret (idempotent
    /// upsert). Rejects an empty secret so the webhook verifier never runs
    /// against a blank key.
    pub async fn connect_github(&self, webhook_secret: &str) -> Result<(), DomainError> {
        if webhook_secret.trim().is_empty() {
            return Err(DomainError::InvalidServiceSecret);
        }
        self.vault.store(GITHUB_SERVICE, webhook_secret).await
    }

    /// Whether GitHub currently has a stored secret. Never returns the secret.
    pub async fn github_connected(&self) -> Result<bool, DomainError> {
        Ok(self.vault.reveal(GITHUB_SERVICE).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockVault {
        secrets: Mutex<HashMap<String, String>>,
    }

    #[async_trait]
    impl SecretVault for MockVault {
        async fn store(&self, service: &str, secret: &str) -> Result<(), DomainError> {
            self.secrets
                .lock()
                .unwrap()
                .insert(service.to_string(), secret.to_string());
            Ok(())
        }
        async fn reveal(&self, service: &str) -> Result<Option<String>, DomainError> {
            Ok(self.secrets.lock().unwrap().get(service).cloned())
        }
    }

    #[tokio::test]
    async fn connect_then_status_is_connected_and_stores_the_secret() {
        let vault = Arc::new(MockVault::default());
        let uc = ServiceConnectionUseCase::new(vault.clone());

        assert!(!uc.github_connected().await.unwrap());
        uc.connect_github("hmac-secret").await.unwrap();
        assert!(uc.github_connected().await.unwrap());
        assert_eq!(
            vault.reveal(GITHUB_SERVICE).await.unwrap().as_deref(),
            Some("hmac-secret")
        );
    }

    #[tokio::test]
    async fn empty_secret_is_rejected_and_stores_nothing() {
        let vault = Arc::new(MockVault::default());
        let uc = ServiceConnectionUseCase::new(vault.clone());

        let err = uc.connect_github("   ").await.unwrap_err();
        assert_eq!(err, DomainError::InvalidServiceSecret);
        assert!(!uc.github_connected().await.unwrap());
    }
}
