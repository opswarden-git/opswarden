// --- server/src/app/release/create_release.rs ---

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::release::Release;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{EventPublisher, ReleaseRepo, TeamRepo};

use super::{load_detail, ReleaseDetail};

pub struct CreateReleaseCommand {
    pub team_id: Uuid,
    pub title: String,
    pub steps: Vec<String>,
    pub requester_id: Uuid,
}

pub struct CreateReleaseUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl CreateReleaseUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        releases: Arc<dyn ReleaseRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            releases,
            events,
        }
    }

    pub async fn create(&self, cmd: CreateReleaseCommand) -> Result<ReleaseDetail, DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(role).can_create_release {
            return Err(DomainError::Forbidden);
        }

        let release = Release::new(cmd.team_id, cmd.title, cmd.steps)?;
        self.releases.save_release(&release).await?;
        self.events
            .publish(DomainEvent::ReleaseStateChanged {
                team_id: release.team_id,
                release_id: release.id,
                new_state: release.base_state,
            })
            .await;
        load_detail(&self.releases, release).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockEventPublisher;
    use crate::app::incident::tests::MockTeamRepo;
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::event::DomainEvent;
    use crate::domain::release::ReleaseState;

    #[tokio::test]
    async fn manager_can_create_a_release() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Manager));
        let releases = Arc::new(MockReleaseRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = CreateReleaseUseCase::new(teams, releases.clone(), events.clone());

        let detail = uc
            .create(CreateReleaseCommand {
                team_id,
                title: "v1.0.0".to_string(),
                steps: vec!["build".into(), "prod".into()],
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(detail.effective_state, ReleaseState::Created);
        assert_eq!(detail.release.steps.len(), 2);
        assert_eq!(releases.releases.lock().unwrap().len(), 1);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::ReleaseStateChanged {
                new_state: ReleaseState::Created,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn responder_cannot_create_a_release() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let releases = Arc::new(MockReleaseRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = CreateReleaseUseCase::new(teams, releases.clone(), events);

        let err = uc
            .create(CreateReleaseCommand {
                team_id,
                title: "v1.0.0".to_string(),
                steps: vec!["build".into()],
                requester_id: requester,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
        assert!(releases.releases.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn observer_cannot_create_a_release() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Observer));
        let releases = Arc::new(MockReleaseRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = CreateReleaseUseCase::new(teams, releases.clone(), events.clone());

        let err = uc
            .create(CreateReleaseCommand {
                team_id,
                title: "v1.0.0".to_string(),
                steps: vec!["build".into()],
                requester_id: requester,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
        assert!(releases.releases.lock().unwrap().is_empty());
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_non_member_cannot_create_a_release() {
        let teams = Arc::new(MockTeamRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = CreateReleaseUseCase::new(teams, releases.clone(), events);

        let err = uc
            .create(CreateReleaseCommand {
                team_id: Uuid::new_v4(),
                title: "v1.0.0".to_string(),
                steps: vec!["build".into()],
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }
}
