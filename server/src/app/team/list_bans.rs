// --- server/src/app/team/list_bans.rs ---
//
// Manager-only listing of a team's bans, for the moderation view. Returns the
// raw ban records (active and expired); the caller can compute current status
// via `TeamBan::is_active`.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
#[cfg(test)]
use crate::domain::team::Role;
use crate::domain::team::TeamBanView;
use crate::ports::TeamRepo;

pub struct ListBansCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListBansResult {
    pub bans: Vec<TeamBanView>,
}

pub struct ListBansUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl ListBansUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn list_bans(&self, cmd: ListBansCommand) -> Result<ListBansResult, DomainError> {
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(requester_role).can_manage_members {
            return Err(DomainError::NotManager);
        }

        let bans = self.teams.list_bans(cmd.team_id).await?;
        Ok(ListBansResult { bans })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;
    use crate::domain::team::TeamBan;

    #[tokio::test]
    async fn manager_lists_the_bans() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let banned = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_ban(TeamBan::permanent(team, banned, manager, None)),
        );
        let use_case = ListBansUseCase::new(repo.clone());

        let result = use_case
            .list_bans(ListBansCommand {
                team_id: team,
                requester_id: manager,
            })
            .await
            .unwrap();

        assert_eq!(result.bans.len(), 1);
        assert_eq!(result.bans[0].ban.user_id, banned);
    }

    #[tokio::test]
    async fn non_manager_cannot_list_bans() {
        let team = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(observer, Role::Observer));
        let use_case = ListBansUseCase::new(repo.clone());

        let result = use_case
            .list_bans(ListBansCommand {
                team_id: team,
                requester_id: observer,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
    }
}
