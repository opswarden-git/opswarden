// --- server/src/app/team/set_member_role.rs ---
//
// Change a team member's role between Observer and Responder. Manager-only.
// The Manager seat is never touched here (promotion is a transfer, and the
// sitting Manager cannot be demoted through this path) — that authority stays
// in `transfer_manager`, preserving the single-Manager invariant.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::{validate_member_role_change, Role};
use crate::ports::TeamRepo;

pub struct SetMemberRoleCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub target_user_id: Uuid,
    pub new_role: Role,
}

pub struct SetMemberRoleUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl SetMemberRoleUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn set_member_role(&self, cmd: SetMemberRoleCommand) -> Result<(), DomainError> {
        // Gate on the requester being a Manager *before* revealing whether the
        // target exists, so a non-manager cannot probe membership.
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if requester_role != Role::Manager {
            return Err(DomainError::NotManager);
        }

        let target_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.target_user_id)
            .await?
            .ok_or(DomainError::MemberNotFound)?;

        validate_member_role_change(requester_role, target_role, cmd.new_role)?;

        self.teams
            .set_member_role(cmd.team_id, cmd.target_user_id, cmd.new_role)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;

    #[tokio::test]
    async fn manager_promotes_observer_to_responder() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(observer, Role::Observer),
        );
        let use_case = SetMemberRoleUseCase::new(repo.clone());

        use_case
            .set_member_role(SetMemberRoleCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: observer,
                new_role: Role::Responder,
            })
            .await
            .unwrap();

        assert_eq!(
            repo.role_changes.lock().unwrap().as_slice(),
            &[(team, observer, Role::Responder)]
        );
    }

    #[tokio::test]
    async fn manager_demotes_responder_to_observer() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(responder, Role::Responder),
        );
        let use_case = SetMemberRoleUseCase::new(repo.clone());

        use_case
            .set_member_role(SetMemberRoleCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: responder,
                new_role: Role::Observer,
            })
            .await
            .unwrap();

        assert_eq!(
            repo.role_changes.lock().unwrap().as_slice(),
            &[(team, responder, Role::Observer)]
        );
    }

    #[tokio::test]
    async fn non_manager_cannot_change_a_role() {
        let team = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(responder, Role::Responder)
                .with_member(observer, Role::Observer),
        );
        let use_case = SetMemberRoleUseCase::new(repo.clone());

        let result = use_case
            .set_member_role(SetMemberRoleCommand {
                team_id: team,
                requester_id: responder,
                target_user_id: observer,
                new_role: Role::Responder,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
        assert!(repo.role_changes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unknown_target_is_member_not_found() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let use_case = SetMemberRoleUseCase::new(repo.clone());

        let result = use_case
            .set_member_role(SetMemberRoleCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: Uuid::new_v4(),
                new_role: Role::Responder,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::MemberNotFound);
        assert!(repo.role_changes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_manager_cannot_be_targeted() {
        // The manager targets themselves (the only Manager) — refused, since it
        // would otherwise break the single-Manager invariant.
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let use_case = SetMemberRoleUseCase::new(repo.clone());

        let result = use_case
            .set_member_role(SetMemberRoleCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: manager,
                new_role: Role::Responder,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::CannotChangeManagerRole);
        assert!(repo.role_changes.lock().unwrap().is_empty());
    }
}
