// --- server/src/app/release/link_incident.rs ---
//
// Link / unlink an incident to a release. Linking an *active* incident to an
// in-progress release flips its effective state to `blocked` and emits
// `release_state_changed`; unlinking the last active incident unblocks it.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{EventPublisher, IncidentRepo, ReleaseRepo, TeamRepo};

use super::{emit_if_state_changed, ReleaseDetail};

pub struct LinkIncidentCommand {
    pub release_id: Uuid,
    pub incident_id: Uuid,
    pub requester_id: Uuid,
}

pub struct LinkIncidentUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl LinkIncidentUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        releases: Arc<dyn ReleaseRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            incidents,
            releases,
            events,
        }
    }

    pub async fn link(&self, cmd: LinkIncidentCommand) -> Result<ReleaseDetail, DomainError> {
        let release = self
            .releases
            .find_release_by_id(cmd.release_id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;

        let role = self
            .teams
            .find_member_role(release.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(role).can_link_release_incident {
            return Err(DomainError::Forbidden);
        }

        // The incident must exist and belong to the release's team; a foreign or
        // missing incident is reported as not-found (no cross-team leak).
        self.incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .filter(|incident| incident.team_id == release.team_id)
            .ok_or(DomainError::IncidentNotFound)?;

        let old_effective = release.effective_state(self.has_active(release.id).await?);
        self.releases
            .link_incident(release.id, cmd.incident_id)
            .await?;
        let release = self
            .releases
            .find_release_by_id(release.id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;
        let new_effective = release.effective_state(self.has_active(release.id).await?);

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

    async fn has_active(&self, release_id: Uuid) -> Result<bool, DomainError> {
        Ok(self
            .releases
            .count_active_linked_incidents(release_id)
            .await?
            > 0)
    }
}

pub struct UnlinkIncidentCommand {
    pub release_id: Uuid,
    pub incident_id: Uuid,
    pub requester_id: Uuid,
}

pub struct UnlinkIncidentUseCase {
    teams: Arc<dyn TeamRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl UnlinkIncidentUseCase {
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

    pub async fn unlink(&self, cmd: UnlinkIncidentCommand) -> Result<ReleaseDetail, DomainError> {
        let release = self
            .releases
            .find_release_by_id(cmd.release_id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;

        let role = self
            .teams
            .find_member_role(release.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(role).can_link_release_incident {
            return Err(DomainError::Forbidden);
        }

        let old_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let old_effective = release.effective_state(old_active);
        self.releases
            .unlink_incident(release.id, cmd.incident_id)
            .await?;
        let release = self
            .releases
            .find_release_by_id(release.id)
            .await?
            .ok_or(DomainError::ReleaseNotFound)?;
        let new_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let new_effective = release.effective_state(new_active);

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
    use crate::app::incident::tests::{MockEventPublisher, MockIncidentRepo, MockTeamRepo};
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::event::DomainEvent;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::release::{Release, ReleaseState};

    #[tokio::test]
    async fn linking_an_active_incident_blocks_an_in_progress_release() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let mut release = Release::new(team_id, "v1", vec!["build".into(), "prod".into()]).unwrap();
        release.validate_step("build", requester, false).unwrap(); // in_progress
        let release_id = release.id;
        let mut incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
        incident.acknowledge().unwrap(); // active
        let incident_id = incident.id;

        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        releases.mark_active(incident_id);
        let events = Arc::new(MockEventPublisher::default());
        let uc = LinkIncidentUseCase::new(teams, incidents, releases.clone(), events.clone());

        let detail = uc
            .link(LinkIncidentCommand {
                release_id,
                incident_id,
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(detail.effective_state, ReleaseState::Blocked);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::ReleaseStateChanged {
                new_state: ReleaseState::Blocked,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn linking_a_foreign_incident_is_not_found() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let release = Release::new(team_id, "v1", vec!["build".into()]).unwrap();
        let release_id = release.id;
        // Incident in a *different* team.
        let incident = Incident::new(Uuid::new_v4(), "elsewhere", Severity::Low).unwrap();
        let incident_id = incident.id;

        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        let events = Arc::new(MockEventPublisher::default());
        let uc = LinkIncidentUseCase::new(teams, incidents, releases, events);

        let err = uc
            .link(LinkIncidentCommand {
                release_id,
                incident_id,
                requester_id: requester,
            })
            .await
            .unwrap_err();
        assert_eq!(err, DomainError::IncidentNotFound);
    }

    #[tokio::test]
    async fn unlinking_the_last_active_incident_unblocks() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let mut release = Release::new(team_id, "v1", vec!["build".into(), "prod".into()]).unwrap();
        release.validate_step("build", requester, false).unwrap(); // in_progress
        let release_id = release.id;
        let incident_id = Uuid::new_v4();

        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        releases
            .link_incident(release_id, incident_id)
            .await
            .unwrap();
        releases.mark_active(incident_id); // currently blocked
        let events = Arc::new(MockEventPublisher::default());
        let uc = UnlinkIncidentUseCase::new(teams, releases, events.clone());

        let detail = uc
            .unlink(UnlinkIncidentCommand {
                release_id,
                incident_id,
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(detail.effective_state, ReleaseState::InProgress);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::ReleaseStateChanged {
                new_state: ReleaseState::InProgress,
                ..
            }]
        ));
    }
}
