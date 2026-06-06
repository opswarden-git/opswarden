// --- server/src/app/team/leave_team.rs ---
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::TeamRepo;

pub struct LeaveTeamCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct LeaveTeamUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl LeaveTeamUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    /// Leave a team. A Manager cannot leave the team (since they are the only Manager),
    /// they must transfer their role first or delete the team.
    pub async fn leave_team(&self, cmd: LeaveTeamCommand) -> Result<(), DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::MemberNotFound)?;

        if role == Role::Manager {
            return Err(DomainError::ManagerCannotLeave);
        }

        self.teams
            .remove_member(cmd.team_id, cmd.requester_id)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;

    #[tokio::test]
    async fn responder_can_leave_team() {
        let team_id = Uuid::new_v4();
        let responder_id = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(responder_id, Role::Responder));
        let use_case = LeaveTeamUseCase::new(repo.clone());

        use_case
            .leave_team(LeaveTeamCommand {
                team_id,
                requester_id: responder_id,
            })
            .await
            .unwrap();

        let removed = repo.removed.lock().unwrap();
        assert_eq!(removed.as_slice(), &[(team_id, responder_id)]);
    }

    #[tokio::test]
    async fn manager_cannot_leave_team() {
        let team_id = Uuid::new_v4();
        let manager_id = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager_id, Role::Manager));
        let use_case = LeaveTeamUseCase::new(repo.clone());

        let result = use_case
            .leave_team(LeaveTeamCommand {
                team_id,
                requester_id: manager_id,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::ManagerCannotLeave);
    }
}
