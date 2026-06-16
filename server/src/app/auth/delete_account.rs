// --- server/src/app/auth/delete_account.rs ---

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::{TeamRepo, UserRepo};
use std::sync::Arc;
use uuid::Uuid;

pub struct DeleteAccountCommand {
    pub user_id: Uuid,
}

pub struct DeleteAccountUseCase {
    users: Arc<dyn UserRepo + Send + Sync>,
    teams: Arc<dyn TeamRepo + Send + Sync>,
}

impl DeleteAccountUseCase {
    pub fn new(
        users: Arc<dyn UserRepo + Send + Sync>,
        teams: Arc<dyn TeamRepo + Send + Sync>,
    ) -> Self {
        Self { users, teams }
    }

    pub async fn delete_account(&self, cmd: DeleteAccountCommand) -> Result<(), DomainError> {
        let teams = self.teams.list_teams_for_user(cmd.user_id).await?;

        for (team, role) in teams {
            if role == Role::Manager {
                let member_count = self.teams.count_members(team.id).await?;
                if member_count > 1 {
                    // There are other members; we can't orphan them.
                    return Err(DomainError::MustTransferManagerFirst);
                } else {
                    // User is the ONLY member of this team. Delete the team safely.
                    self.teams.delete_team(team.id).await?;
                }
            }
        }

        self.users.delete_account(cmd.user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockTeamRepo;
    use crate::domain::user::User;
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
        async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError> {
            self.deleted.lock().unwrap().push(user_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn managing_a_team_with_others_blocks_account_deletion() {
        let user = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let team = Uuid::new_v4();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team, user, Role::Manager)
                .with_member(team, other_user, Role::Responder),
        );
        let users = Arc::new(SpyUserRepo::default());
        let use_case = DeleteAccountUseCase::new(users.clone(), teams);

        let err = use_case
            .delete_account(DeleteAccountCommand { user_id: user })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::MustTransferManagerFirst);
        assert!(users.deleted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn lone_manager_deletes_team_and_account() {
        let user = Uuid::new_v4();
        let team = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, user, Role::Manager));
        let users = Arc::new(SpyUserRepo::default());
        let use_case = DeleteAccountUseCase::new(users.clone(), teams);

        use_case
            .delete_account(DeleteAccountCommand { user_id: user })
            .await
            .unwrap();

        assert_eq!(users.deleted.lock().unwrap().as_slice(), &[user]);
    }

    #[tokio::test]
    async fn a_plain_member_can_delete_their_account() {
        let user = Uuid::new_v4();
        let team = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, user, Role::Responder));
        let users = Arc::new(SpyUserRepo::default());
        let use_case = DeleteAccountUseCase::new(users.clone(), teams);

        use_case
            .delete_account(DeleteAccountCommand { user_id: user })
            .await
            .unwrap();

        assert_eq!(users.deleted.lock().unwrap().as_slice(), &[user]);
    }
}
