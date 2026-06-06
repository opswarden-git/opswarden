use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::team::Role;
use crate::ports::{EventPublisher, IncidentRepo, TeamRepo};

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
    events: Arc<dyn EventPublisher>,
}

impl ChangeIncidentStatusUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            incidents,
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

        if !role.can_act_as(Role::Responder) {
            return Err(DomainError::Forbidden);
        }

        let changed = match cmd.new_status {
            IncidentStatus::Open => Err(DomainError::InvalidIncidentTransition),
            IncidentStatus::Acknowledged => incident.acknowledge(),
            IncidentStatus::Escalated => incident.escalate(),
            IncidentStatus::Resolved => incident.resolve(),
        }?;

        if changed {
            self.incidents.update_incident(&incident).await?;
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
    use crate::domain::event::DomainEvent;
    use crate::domain::incident::Incident;

    #[tokio::test]
    async fn responder_can_acknowledge_an_incident() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone(), events.clone());

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
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone(), events.clone());

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
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone(), events.clone());

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
    async fn observer_cannot_change_incident_status() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let events = Arc::new(MockEventPublisher::default());
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone(), events.clone());

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
