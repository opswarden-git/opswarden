// --- server/src/app/release/get_release.rs ---

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::{ReleaseRepo, TeamRepo};

use super::{load_detail, ReleaseDetail};

pub struct GetReleaseCommand {
    pub release_id: Uuid,
    pub requester_id: Uuid,
}

pub struct GetReleaseUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
}

impl GetReleaseUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, releases: Arc<dyn ReleaseRepo>) -> Self {
        Self { teams, releases }
    }

    pub async fn get(&self, cmd: GetReleaseCommand) -> Result<ReleaseDetail, DomainError> {
        let release = self
            .releases
            .find_release_by_id(cmd.release_id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;

        self.teams
            .find_member_role(release.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        load_detail(&self.releases, release).await
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
    async fn member_can_get_a_release() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into()]).unwrap();
        let release_id = release.id;
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Observer));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let uc = GetReleaseUseCase::new(teams, releases);

        let detail = uc
            .get(GetReleaseCommand {
                release_id,
                requester_id: requester,
            })
            .await
            .unwrap();
        assert_eq!(detail.release.id, release_id);
    }

    #[tokio::test]
    async fn a_non_member_cannot_get_a_release() {
        let team_id = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into()]).unwrap();
        let release_id = release.id;
        let teams = Arc::new(MockTeamRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let uc = GetReleaseUseCase::new(teams, releases);

        let err = uc
            .get(GetReleaseCommand {
                release_id,
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();
        assert_eq!(err, DomainError::Forbidden);
    }

    #[tokio::test]
    async fn an_unknown_release_is_not_found() {
        let teams = Arc::new(MockTeamRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        let uc = GetReleaseUseCase::new(teams, releases);

        let err = uc
            .get(GetReleaseCommand {
                release_id: Uuid::new_v4(),
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();
        assert_eq!(err, DomainError::ReleaseNotFound);
    }
}
