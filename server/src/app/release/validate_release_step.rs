// --- server/src/app/release/validate_release_step.rs ---

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::team::Role;
use crate::ports::{EventPublisher, ReleaseRepo, TeamRepo};

use super::{emit_if_state_changed, ReleaseDetail};

pub struct ValidateReleaseStepCommand {
    pub release_id: Uuid,
    pub step: String,
    pub requester_id: Uuid,
}

pub struct ValidateReleaseStepUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl ValidateReleaseStepUseCase {
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

    pub async fn validate(
        &self,
        cmd: ValidateReleaseStepCommand,
    ) -> Result<ReleaseDetail, DomainError> {
        let mut release = self
            .releases
            .find_release_by_id(cmd.release_id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;

        let role = self
            .teams
            .find_member_role(release.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !role.can_act_as(Role::Responder) {
            return Err(DomainError::Forbidden);
        }

        // Validation doesn't touch incidents, so `has_active` is the same before
        // and after; the effective state moves only via the base transition.
        let has_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let old_effective = release.effective_state(has_active);

        release.validate_step(&cmd.step, cmd.requester_id, has_active)?;
        self.releases.update_release(&release).await?;

        let new_effective = release.effective_state(has_active);
        self.events
            .publish(DomainEvent::ReleaseStepValidated {
                team_id: release.team_id,
                release_id: release.id,
                step: cmd.step.clone(),
                by: cmd.requester_id,
            })
            .await;
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
    use crate::domain::release::{Release, ReleaseState};

    fn setup(
        role: Role,
    ) -> (
        Uuid,
        Uuid,
        Uuid,
        Arc<MockReleaseRepo>,
        Arc<MockEventPublisher>,
        ValidateReleaseStepUseCase,
    ) {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into(), "prod".into()]).unwrap();
        let release_id = release.id;
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, requester, role));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let events = Arc::new(MockEventPublisher::default());
        let uc = ValidateReleaseStepUseCase::new(teams, releases.clone(), events.clone());
        (team_id, requester, release_id, releases, events, uc)
    }

    #[tokio::test]
    async fn responder_validates_first_step_and_release_goes_in_progress() {
        let (_team, requester, release_id, releases, events, uc) = setup(Role::Responder);

        let detail = uc
            .validate(ValidateReleaseStepCommand {
                release_id,
                step: "build".to_string(),
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(detail.effective_state, ReleaseState::InProgress);
        let published = events.published.lock().unwrap();
        // step_validated, then state_changed (created -> in_progress)
        assert!(matches!(
            published[0],
            DomainEvent::ReleaseStepValidated { .. }
        ));
        assert!(matches!(
            published[1],
            DomainEvent::ReleaseStateChanged {
                new_state: ReleaseState::InProgress,
                ..
            }
        ));
        assert_eq!(
            releases.releases.lock().unwrap()[&release_id].base_state,
            ReleaseState::InProgress
        );
    }

    #[tokio::test]
    async fn out_of_order_step_is_rejected() {
        let (_team, requester, release_id, _releases, events, uc) = setup(Role::Responder);

        let err = uc
            .validate(ValidateReleaseStepCommand {
                release_id,
                step: "prod".to_string(),
                requester_id: requester,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::InvalidReleaseStep);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn observer_cannot_validate_a_step() {
        let (_team, requester, release_id, _releases, events, uc) = setup(Role::Observer);

        let err = uc
            .validate(ValidateReleaseStepCommand {
                release_id,
                step: "build".to_string(),
                requester_id: requester,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn validating_while_blocked_is_refused() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let mut release = Release::new(team_id, "v1", vec!["build".into(), "prod".into()]).unwrap();
        // Make it in_progress by validating the first step in-memory.
        release.validate_step("build", requester, false).unwrap();
        let release_id = release.id;
        let incident = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        releases.link_incident(release_id, incident).await.unwrap();
        releases.mark_active(incident); // an active linked incident -> blocked
        let events = Arc::new(MockEventPublisher::default());
        let uc = ValidateReleaseStepUseCase::new(teams, releases, events.clone());

        let err = uc
            .validate(ValidateReleaseStepCommand {
                release_id,
                step: "prod".to_string(),
                requester_id: requester,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::ReleaseBlocked);
        assert!(events.published.lock().unwrap().is_empty());
    }
}
