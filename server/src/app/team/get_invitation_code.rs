use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::ports::TeamRepo;

pub struct GetInvitationCodeCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct GetInvitationCodeResult {
    pub invitation_code: String,
}

pub struct GetInvitationCodeUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl GetInvitationCodeUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn get(
        &self,
        cmd: GetInvitationCodeCommand,
    ) -> Result<GetInvitationCodeResult, DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(role).can_view_invitation_code {
            return Err(DomainError::NotManager);
        }

        let team = self
            .teams
            .find_team_by_id(cmd.team_id)
            .await?
            .ok_or(DomainError::TeamNotFound)?;
        Ok(GetInvitationCodeResult {
            invitation_code: team.invitation_code.as_str().to_string(),
        })
    }
}
