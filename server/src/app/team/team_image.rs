use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::{Role, TeamImage};
use crate::ports::TeamRepo;

pub struct UpdateTeamImageCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub media_type: String,
    pub content: Vec<u8>,
}

pub struct DeleteTeamImageCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct GetTeamImageCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct UpdateTeamImageUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl UpdateTeamImageUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    async fn require_manager(&self, team_id: Uuid, requester_id: Uuid) -> Result<(), DomainError> {
        match self.teams.find_member_role(team_id, requester_id).await? {
            Some(Role::Manager) => Ok(()),
            _ => Err(DomainError::NotManager),
        }
    }

    pub async fn update(&self, command: UpdateTeamImageCommand) -> Result<TeamImage, DomainError> {
        self.require_manager(command.team_id, command.requester_id)
            .await?;
        let image = TeamImage::new(command.media_type, command.content)?;
        self.teams.save_team_image(command.team_id, &image).await?;
        Ok(image)
    }

    pub async fn delete(&self, command: DeleteTeamImageCommand) -> Result<(), DomainError> {
        self.require_manager(command.team_id, command.requester_id)
            .await?;
        self.teams.delete_team_image(command.team_id).await
    }
}

pub struct GetTeamImageUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl GetTeamImageUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn get(&self, command: GetTeamImageCommand) -> Result<TeamImage, DomainError> {
        if self
            .teams
            .find_member_role(command.team_id, command.requester_id)
            .await?
            .is_none()
        {
            return Err(DomainError::Forbidden);
        }
        self.teams
            .find_team_image_for_member(command.team_id, command.requester_id)
            .await?
            .ok_or(DomainError::TeamImageNotFound)
    }
}
