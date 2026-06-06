use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::team::Role;
use crate::ports::{IncidentRepo, TeamRepo};

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
}

impl ChangeIncidentStatusUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
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

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::incident::Incident;

    #[tokio::test]
    async fn responder_can_acknowledge_an_incident() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone());

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
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone());

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
    }

    #[tokio::test]
    async fn observer_cannot_change_incident_status() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = ChangeIncidentStatusUseCase::new(teams, incidents.clone());

        let result = use_case
            .change_status(ChangeIncidentStatusCommand {
                incident_id: incident.id,
                requester_id,
                new_status: IncidentStatus::Acknowledged,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(incidents.updated.lock().unwrap().is_empty());
    }
}
