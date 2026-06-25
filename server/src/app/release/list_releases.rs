// --- server/src/app/release/list_releases.rs ---

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::{ReleaseRepo, TeamRepo};

use super::{load_detail, ReleaseDetail};

pub struct ListReleasesCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct ListReleasesUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
}

impl ListReleasesUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, releases: Arc<dyn ReleaseRepo>) -> Self {
        Self { teams, releases }
    }

    pub async fn list(&self, cmd: ListReleasesCommand) -> Result<Vec<ReleaseDetail>, DomainError> {
        // Any member (Observer included) may read; non-members are forbidden.
        self.teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let releases = self.releases.list_releases_for_team(cmd.team_id).await?;
        let mut out = Vec::with_capacity(releases.len());
        for release in releases {
            out.push(load_detail(&self.releases, release).await?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockTeamRepo;
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::release::Release;
    use crate::domain::team::Role;

    #[tokio::test]
    async fn an_observer_can_list_releases() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Observer));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(Release::new(team_id, "v1", vec!["build".into()]).unwrap());
        let uc = ListReleasesUseCase::new(teams, releases);

        let out = uc
            .list(ListReleasesCommand {
                team_id,
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(out.len(), 1);
    }

    #[tokio::test]
    async fn a_non_member_cannot_list_releases() {
        let teams = Arc::new(MockTeamRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        let uc = ListReleasesUseCase::new(teams, releases);

        let err = uc
            .list(ListReleasesCommand {
                team_id: Uuid::new_v4(),
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }
}
