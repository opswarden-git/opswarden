use std::sync::Arc;

use uuid::Uuid;

use crate::app::release::{emit_release_state_changes, snapshot_linked_releases};
use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::incident_event::IncidentEvent;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{EventPublisher, IncidentRepo, ReleaseRepo, TeamRepo};

pub struct ChangeIncidentStatusCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub new_status: IncidentStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChangeIncidentStatusResult {
    pub incident_id: Uuid,
    pub status: IncidentStatus,
    pub severity: Severity,
    pub changed: bool,
}

pub struct ChangeIncidentStatusUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    releases: Arc<dyn ReleaseRepo>,
    events: Arc<dyn EventPublisher>,
}

impl ChangeIncidentStatusUseCase {
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

    pub async fn change_status(
        &self,
        cmd: ChangeIncidentStatusCommand,
    ) -> Result<ChangeIncidentStatusResult, DomainError> {
        let mut incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        let role = self
            .teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        if !derive_capabilities(role).can_transition_incident {
            return Err(DomainError::Forbidden);
        }

        let previous_status = incident.status;
        let changed = match cmd.new_status {
            IncidentStatus::Open => Err(DomainError::InvalidIncidentTransition),
            IncidentStatus::Acknowledged => incident.acknowledge(),
            IncidentStatus::Escalated => incident.escalate(),
            IncidentStatus::Resolved => incident.resolve(),
        }?;

        if changed {
            // Snapshot the effective state of releases linked to this incident
            // *before* persisting the status change, so we can detect an
            // auto-(un)block afterwards (notably: resolving frees a blocked
            // release). The snapshot reads the still-old status from the DB.
            let release_snapshot = snapshot_linked_releases(&self.releases, incident.id).await?;

            let event = IncidentEvent::status_changed(
                incident.id,
                cmd.requester_id,
                previous_status,
                incident.status,
            );
            self.incidents
                .update_incident_with_event(&incident, &event)
                .await?;
            self.events
                .publish(DomainEvent::IncidentStateChanged {
                    team_id: incident.team_id,
                    incident_id: incident.id,
                    new_status: incident.status,
                    by: cmd.requester_id,
                })
                .await;
            if incident.status == IncidentStatus::Escalated {
                self.events
                    .publish(DomainEvent::IncidentEscalated {
                        team_id: incident.team_id,
                        incident_id: incident.id,
                        new_severity: incident.severity,
                        by: cmd.requester_id,
                    })
                    .await;
            }

            // Re-evaluate the snapshotted releases against the now-persisted
            // status and emit `release_state_changed` for any that moved.
            emit_release_state_changes(&self.releases, &self.events, release_snapshot).await?;
        }

        Ok(ChangeIncidentStatusResult {
            incident_id: incident.id,
            status: incident.status,
            severity: incident.severity,
            changed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockEventPublisher, MockIncidentRepo, MockTeamRepo};
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::event::DomainEvent;
    use crate::domain::incident::Incident;
    use crate::domain::release::{Release, ReleaseState};

    #[tokio::test]
    async fn responder_can_acknowledge_an_incident() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(
            teams,
            incidents.clone(),
            Arc::new(MockReleaseRepo::default()),
            events.clone(),
        );

        let result = use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id: incident.id,
                requester_id,
                new_status: IncidentStatus::Acknowledged,
            })
            .await
            .unwrap();

        assert_eq!(result.status, IncidentStatus::Acknowledged);
        assert_eq!(incidents.updated.lock().unwrap().len(), 1);
        assert_eq!(incidents.incident_events.lock().unwrap().len(), 1);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::IncidentStateChanged {
                new_status: IncidentStatus::Acknowledged,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn escalation_emits_state_changed_and_escalated() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let mut incident = Incident::new(team_id, "Worker panic", Severity::Critical).unwrap();
        incident.acknowledge().unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(
            teams,
            incidents.clone(),
            Arc::new(MockReleaseRepo::default()),
            events.clone(),
        );

        use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id: incident.id,
                requester_id,
                new_status: IncidentStatus::Escalated,
            })
            .await
            .unwrap();

        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [
                DomainEvent::IncidentStateChanged {
                    new_status: IncidentStatus::Escalated,
                    ..
                },
                DomainEvent::IncidentEscalated {
                    new_severity: Severity::Critical,
                    ..
                }
            ]
        ));
    }

    #[tokio::test]
    async fn resolve_is_idempotent_when_already_resolved() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let mut incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        incident.acknowledge().unwrap();
        incident.resolve().unwrap();

        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Manager));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(
            teams,
            incidents.clone(),
            Arc::new(MockReleaseRepo::default()),
            events.clone(),
        );

        let result = use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id: incident.id,
                requester_id,
                new_status: IncidentStatus::Resolved,
            })
            .await
            .unwrap();

        assert!(!result.changed);
        assert!(incidents.updated.lock().unwrap().is_empty());
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn resolving_an_incident_unblocks_its_linked_release() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let mut incident = Incident::new(team_id, "blocking incident", Severity::High).unwrap();
        incident.acknowledge().unwrap(); // active
        let incident_id = incident.id;

        // A linked, in-progress release that is currently blocked by the incident.
        let mut release = Release::new(team_id, "v1", vec!["build".into(), "prod".into()]).unwrap();
        release.validate_step("build", requester_id, false).unwrap(); // -> in_progress
        let release_id = release.id;

        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        releases
            .link_incident(release_id, incident_id)
            .await
            .unwrap();
        // Active count is 1 before the resolve, 0 after — drive the recompute.
        releases.script_count(release_id, vec![1, 0]);
        let events = Arc::new(MockEventPublisher::default());
        let use_case =
            ChangeIncidentStatusUseCase::new(teams, incidents.clone(), releases, events.clone());

        use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id,
                requester_id,
                new_status: IncidentStatus::Resolved,
            })
            .await
            .unwrap();

        // The incident state change, then the auto-unblock of the release.
        let published = events.published.lock().unwrap();
        assert!(published.iter().any(|e| matches!(
            e,
            DomainEvent::ReleaseStateChanged {
                release_id: r,
                new_state: ReleaseState::InProgress,
                ..
            } if *r == release_id
        )));
    }

    #[tokio::test]
    async fn observer_cannot_change_incident_status() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(
            teams,
            incidents.clone(),
            Arc::new(MockReleaseRepo::default()),
            events.clone(),
        );

        let result = use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id: incident.id,
                requester_id,
                new_status: IncidentStatus::Acknowledged,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(incidents.updated.lock().unwrap().is_empty());
        assert!(events.published.lock().unwrap().is_empty());
    }
}
