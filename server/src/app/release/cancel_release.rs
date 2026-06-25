// --- server/src/app/release/cancel_release.rs ---

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::{EventPublisher, ReleaseRepo, TeamRepo};

use super::{emit_if_state_changed, ReleaseDetail};

pub struct CancelReleaseCommand {
    pub release_id: Uuid,
    pub requester_id: Uuid,
}

pub struct CancelReleaseUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl CancelReleaseUseCase {
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

    pub async fn cancel(&self, cmd: CancelReleaseCommand) -> Result<ReleaseDetail, DomainError> {
        let mut release = self
            .releases
            .find_release_by_id(cmd.release_id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;

        // Cancelling is a destructive action — Manager only.
        let role = self
            .teams
            .find_member_role(release.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if role != Role::Manager {
            return Err(DomainError::NotManager);
        }

        let has_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let old_effective = release.effective_state(has_active);

        release.cancel()?;
        self.releases.update_release(&release).await?;

        let new_effective = release.effective_state(has_active);
        emit_if_state_changed(
            &self.events,
            release.team_id,
            release.id,
            old_effective,
            new_effective,
        )
        .await;

        let linked_incident_ids = self.releases.list_linked_incident_ids(release.id).await?;
        Ok(ReleaseDetail {
            release,
            effective_state: new_effective,
            linked_incident_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockEventPublisher, MockTeamRepo};
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::event::DomainEvent;
    use crate::domain::release::{Release, ReleaseState};

    #[tokio::test]
    async fn manager_can_cancel_a_release() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into()]).unwrap();
        let release_id = release.id;
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, manager, Role::Manager));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let events = Arc::new(MockEventPublisher::default());
        let uc = CancelReleaseUseCase::new(teams, releases, events.clone());

        let detail = uc
            .cancel(CancelReleaseCommand {
                release_id,
                requester_id: manager,
            })
            .await
            .unwrap();

        assert_eq!(detail.effective_state, ReleaseState::Cancelled);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::ReleaseStateChanged {
                new_state: ReleaseState::Cancelled,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn a_responder_cannot_cancel_a_release() {
        let team_id = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into()]).unwrap();
        let release_id = release.id;
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, responder, Role::Responder));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let events = Arc::new(MockEventPublisher::default());
        let uc = CancelReleaseUseCase::new(teams, releases, events.clone());

        let err = uc
            .cancel(CancelReleaseCommand {
                release_id,
                requester_id: responder,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::NotManager);
        assert!(events.published.lock().unwrap().is_empty());
    }
}
