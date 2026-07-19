use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::ports::TeamRepo;

pub struct UnbanMemberCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub target_user_id: Uuid,
}

pub struct UnbanMemberUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl UnbanMemberUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn unban(&self, cmd: UnbanMemberCommand) -> Result<(), DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(role).can_manage_members {
            return Err(DomainError::NotManager);
        }

        self.teams.remove_ban(cmd.team_id, cmd.target_user_id).await
    }
}
