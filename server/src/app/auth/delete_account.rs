// --- server/src/app/auth/delete_account.rs ---

use crate::domain::error::DomainError;
use crate::ports::UserRepo;
use std::sync::Arc;
use uuid::Uuid;

pub struct DeleteAccountCommand {
    pub user_id: Uuid,
}

pub struct DeleteAccountUseCase {
    users: Arc<dyn UserRepo + Send + Sync>,
}

impl DeleteAccountUseCase {
    pub fn new(users: Arc<dyn UserRepo + Send + Sync>) -> Self {
        Self { users }
    }

    pub async fn delete_account(&self, cmd: DeleteAccountCommand) -> Result<(), DomainError> {
        self.users.delete_account(cmd.user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{Locale, User};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct SpyUserRepo {
        deleted: Mutex<Vec<Uuid>>,
    }

    #[async_trait]
    impl UserRepo for SpyUserRepo {
        async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, DomainError> {
            Ok(None)
        }
        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
            Ok(None)
        }
        async fn save(&self, _user: &User) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update_locale(&self, _user_id: Uuid, _locale: Locale) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError> {
            self.deleted.lock().unwrap().push(user_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn delegates_the_atomic_account_command() {
        let user = Uuid::new_v4();
        let users = Arc::new(SpyUserRepo::default());
        let use_case = DeleteAccountUseCase::new(users.clone());

        use_case
            .delete_account(DeleteAccountCommand { user_id: user })
            .await
            .unwrap();

        assert_eq!(users.deleted.lock().unwrap().as_slice(), &[user]);
    }
}
