// --- server/src/app/team/list_teams.rs ---
//
// Read use-case: every team the requester belongs to, with the role they hold.
// No extra RBAC gate — the repository query is already scoped to the user id, so
// a user only ever sees their own memberships.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::TeamRepo;

pub struct ListTeamsCommand {
    pub user_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TeamSummary {
    pub team_id: Uuid,
    pub name: String,
    pub invitation_code: String,
    pub role: Role,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListTeamsResult {
    pub teams: Vec<TeamSummary>,
}

pub struct ListTeamsUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl ListTeamsUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn list_teams(&self, cmd: ListTeamsCommand) -> Result<ListTeamsResult, DomainError> {
        let teams = self.teams.list_teams_for_user(cmd.user_id).await?;
        Ok(ListTeamsResult {
            teams: teams
                .into_iter()
                .map(|(team, role)| TeamSummary {
                    team_id: team.id,
                    name: team.name,
                    invitation_code: team.invitation_code.as_str().to_string(),
                    role,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockTeamRepo;

    #[tokio::test]
    async fn lists_only_the_users_own_teams() {
        let user = Uuid::new_v4();
        let other = Uuid::new_v4();
        let team_a = Uuid::new_v4();
        let team_b = Uuid::new_v4();
        let foreign = Uuid::new_v4();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team_a, user, Role::Manager)
                .with_member(team_b, user, Role::Observer)
                .with_member(foreign, other, Role::Manager),
        );
        let use_case = ListTeamsUseCase::new(teams);

        let result = use_case
            .list_teams(ListTeamsCommand { user_id: user })
            .await
            .unwrap();

        assert_eq!(result.teams.len(), 2);
        let ids: Vec<_> = result.teams.iter().map(|t| t.team_id).collect();
        assert!(ids.contains(&team_a));
        assert!(ids.contains(&team_b));
        assert!(!ids.contains(&foreign));
    }
}
