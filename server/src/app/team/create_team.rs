// --- server/src/app/team/create_team.rs ---
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::{Role, Team};
use crate::ports::TeamRepo;

pub struct CreateTeamCommand {
    pub name: String,
    pub creator_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateTeamResult {
    pub team_id: Uuid,
    pub name: String,
    pub invitation_code: String,
}

pub struct CreateTeamUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl CreateTeamUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    /// Create a team and make the creator its sole Manager. The single-Manager
    /// invariant is born here: a team never exists without exactly one Manager.
    pub async fn create_team(
        &self,
        cmd: CreateTeamCommand,
    ) -> Result<CreateTeamResult, DomainError> {
        let team = Team::new(cmd.name)?;
        self.teams.save_team(&team).await?;
        self.teams
            .add_member(team.id, cmd.creator_id, Role::Manager)
            .await?;
        Ok(CreateTeamResult {
            team_id: team.id,
            name: team.name,
            invitation_code: team.invitation_code.as_str().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;

    #[tokio::test]
    async fn create_team_makes_creator_the_manager() {
        let repo = Arc::new(MockTeamRepo::default());
        let creator = Uuid::new_v4();
        let use_case = CreateTeamUseCase::new(repo.clone());

        let result = use_case
            .create_team(CreateTeamCommand {
                name: "SRE Core".to_string(),
                creator_id: creator,
            })
            .await
            .unwrap();

        assert_eq!(result.name, "SRE Core");
        assert!(result.invitation_code.starts_with("OPS-"));
        let added = repo.added.lock().unwrap();
        assert_eq!(
            added.as_slice(),
            &[(result.team_id, creator, Role::Manager)]
        );
    }

    #[tokio::test]
    async fn create_team_rejects_blank_name() {
        let repo = Arc::new(MockTeamRepo::default());
        let use_case = CreateTeamUseCase::new(repo.clone());

        let result = use_case
            .create_team(CreateTeamCommand {
                name: "  ".to_string(),
                creator_id: Uuid::new_v4(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidTeamName);
        assert!(repo.added.lock().unwrap().is_empty());
    }
}
