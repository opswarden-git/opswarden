// --- server/src/app/team/delete_team.rs ---
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::TeamRepo;

pub struct DeleteTeamCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct DeleteTeamUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl DeleteTeamUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    /// Delete a team. Enforces RBAC: only a Manager can delete the team.
    pub async fn delete_team(&self, cmd: DeleteTeamCommand) -> Result<(), DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::NotManager)?;

        if !derive_capabilities(role).can_delete_team {
            return Err(DomainError::NotManager);
        }

        self.teams.delete_team(cmd.team_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;

    #[tokio::test]
    async fn manager_can_delete_team() {
        let team_id = Uuid::new_v4();
        let manager_id = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager_id, Role::Manager));
        let use_case = DeleteTeamUseCase::new(repo.clone());

        use_case
            .delete_team(DeleteTeamCommand {
                team_id,
                requester_id: manager_id,
            })
            .await
            .unwrap();

        let deleted = repo.deleted.lock().unwrap();
        assert_eq!(deleted.as_slice(), &[team_id]);
    }

    #[tokio::test]
    async fn responder_cannot_delete_team() {
        let team_id = Uuid::new_v4();
        let responder_id = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(responder_id, Role::Responder));
        let use_case = DeleteTeamUseCase::new(repo.clone());

        let result = use_case
            .delete_team(DeleteTeamCommand {
                team_id,
                requester_id: responder_id,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
    }
}
