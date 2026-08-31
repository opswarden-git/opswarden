use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::{Clock, TeamRepo, UserRepo};

pub struct AddMemberCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub target_user_id: Uuid,
}

pub struct AddMemberUseCase {
    teams: Arc<dyn TeamRepo>,
    users: Arc<dyn UserRepo>,
    clock: Arc<dyn Clock>,
}

impl AddMemberUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, users: Arc<dyn UserRepo>, clock: Arc<dyn Clock>) -> Self {
        Self {
            teams,
            users,
            clock,
        }
    }

    /// Add an existing account as an Observer. Only a Manager may bypass the
    /// invitation-code flow, and an active ban remains authoritative.
    pub async fn add_member(&self, cmd: AddMemberCommand) -> Result<(), DomainError> {
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(requester_role).can_manage_members {
            return Err(DomainError::NotManager);
        }

        if self.users.find_by_id(cmd.target_user_id).await?.is_none() {
            return Err(DomainError::UserNotFound);
        }
        if self
            .teams
            .find_member_role(cmd.team_id, cmd.target_user_id)
            .await?
            .is_some()
        {
            return Err(DomainError::AlreadyMember);
        }
        if let Some(ban) = self.teams.find_ban(cmd.team_id, cmd.target_user_id).await? {
            if ban.is_active(self.clock.now()) {
                return Err(DomainError::UserBanned);
            }
        }

        self.teams
            .add_member(cmd.team_id, cmd.target_user_id, Role::Observer)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::clock::SystemClock;
    use crate::app::auth::tests::MockUserRepo;
    use crate::app::team::tests::MockTeamRepo;
    use crate::domain::team::TeamBan;

    fn clock() -> Arc<SystemClock> {
        Arc::new(SystemClock)
    }

    fn users_with(_user_id: Uuid) -> Arc<MockUserRepo> {
        Arc::new(MockUserRepo {
            simulate_user_exists: true,
        })
    }

    #[tokio::test]
    async fn manager_adds_an_existing_user_as_observer() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let target = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let use_case = AddMemberUseCase::new(teams.clone(), users_with(target), clock());

        use_case
            .add_member(AddMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: target,
            })
            .await
            .unwrap();

        assert_eq!(
            teams.added.lock().unwrap().as_slice(),
            &[(team, target, Role::Observer)]
        );
    }

    #[tokio::test]
    async fn responder_cannot_add_a_member() {
        let team = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let target = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(responder, Role::Responder));
        let use_case = AddMemberUseCase::new(teams.clone(), users_with(target), clock());

        let result = use_case
            .add_member(AddMemberCommand {
                team_id: team,
                requester_id: responder,
                target_user_id: target,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
        assert!(teams.added.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn existing_or_banned_users_cannot_be_added() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let target = Uuid::new_v4();
        let existing = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(target, Role::Observer),
        );
        let result = AddMemberUseCase::new(existing, users_with(target), clock())
            .add_member(AddMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: target,
            })
            .await;
        assert_eq!(result.unwrap_err(), DomainError::AlreadyMember);

        let banned = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_ban(TeamBan::permanent(team, target, manager, None)),
        );
        let result = AddMemberUseCase::new(banned, users_with(target), clock())
            .add_member(AddMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: target,
            })
            .await;
        assert_eq!(result.unwrap_err(), DomainError::UserBanned);
    }
}
