// --- server/src/app/team/list_members.rs ---
//
// Read use-case: the roster of a team (its members + roles), for the team
// detail view. Gated by membership — only someone who belongs to the team may
// see who else is on it. A non-member is `Forbidden`, never a silent leak of
// the roster. No role gate beyond membership: an Observer may read the roster.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::TeamMemberView;
use crate::ports::TeamRepo;

pub struct ListTeamMembersCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListTeamMembersResult {
    pub members: Vec<TeamMemberView>,
}

pub struct ListTeamMembersUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl ListTeamMembersUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    pub async fn list_members(
        &self,
        cmd: ListTeamMembersCommand,
    ) -> Result<ListTeamMembersResult, DomainError> {
        // RBAC: the requester must belong to the team to see its roster.
        self.teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let members = self.teams.list_members(cmd.team_id).await?;
        Ok(ListTeamMembersResult { members })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockTeamRepo;
    use crate::domain::team::Role;

    #[tokio::test]
    async fn a_member_sees_the_full_roster() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team, manager, Role::Manager)
                .with_member(team, observer, Role::Observer),
        );
        let use_case = ListTeamMembersUseCase::new(teams);

        let result = use_case
            .list_members(ListTeamMembersCommand {
                team_id: team,
                requester_id: observer,
            })
            .await
            .unwrap();

        assert_eq!(result.members.len(), 2);
        let ids: Vec<_> = result.members.iter().map(|m| m.user_id).collect();
        assert!(ids.contains(&manager));
        assert!(ids.contains(&observer));
        assert!(result.members.iter().all(|m| m.email.contains('@')));
    }

    #[tokio::test]
    async fn a_non_member_is_forbidden() {
        let team = Uuid::new_v4();
        let member = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, member, Role::Manager));
        let use_case = ListTeamMembersUseCase::new(teams);

        let result = use_case
            .list_members(ListTeamMembersCommand {
                team_id: team,
                requester_id: stranger,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
    }

    #[tokio::test]
    async fn members_of_another_team_are_not_leaked() {
        // Cross-team scoping: requester belongs to `mine` and asks for `mine`,
        // so the result must contain only `mine`'s members, never `other`'s.
        let mine = Uuid::new_v4();
        let other = Uuid::new_v4();
        let me = Uuid::new_v4();
        let foreign = Uuid::new_v4();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(mine, me, Role::Manager)
                .with_member(other, foreign, Role::Manager),
        );
        let use_case = ListTeamMembersUseCase::new(teams);

        let result = use_case
            .list_members(ListTeamMembersCommand {
                team_id: mine,
                requester_id: me,
            })
            .await
            .unwrap();

        let ids: Vec<_> = result.members.iter().map(|m| m.user_id).collect();
        assert_eq!(ids, vec![me]);
        assert!(!ids.contains(&foreign));
    }
}
