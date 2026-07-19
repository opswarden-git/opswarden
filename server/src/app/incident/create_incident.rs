use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::incident_event::IncidentEvent;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct CreateIncidentCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub title: String,
    pub description: String,
    pub severity: Severity,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateIncidentResult {
    pub incident_id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub severity: Severity,
}

pub struct CreateIncidentUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl CreateIncidentUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    pub async fn create_incident(
        &self,
        cmd: CreateIncidentCommand,
    ) -> Result<CreateIncidentResult, DomainError> {
        let role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        if !derive_capabilities(role).can_create_incident {
            return Err(DomainError::Forbidden);
        }

        let incident = Incident::new_by(
            cmd.team_id,
            cmd.title,
            cmd.description,
            cmd.severity,
            cmd.requester_id,
        )?;
        let event = IncidentEvent::created(&incident, Some(cmd.requester_id));
        self.incidents
            .save_incident_with_event(&incident, &event)
            .await?;

        Ok(CreateIncidentResult {
            incident_id: incident.id,
            team_id: incident.team_id,
            title: incident.title,
            status: incident.status,
            severity: incident.severity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};

    #[tokio::test]
    async fn manager_can_create_an_incident() {
        let requester_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Manager));
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = CreateIncidentUseCase::new(teams, incidents.clone());

        let result = use_case
            .create_incident(CreateIncidentCommand {
                team_id,
                requester_id,
                title: "Primary DB latency".to_string(),
                description: "Latency exceeds the customer SLO".to_string(),
                severity: Severity::High,
            })
            .await
            .unwrap();

        assert_eq!(result.status, IncidentStatus::Open);
        assert_eq!(result.severity, Severity::High);
        assert_eq!(incidents.saved.lock().unwrap().len(), 1);
        assert!(matches!(
            incidents.incident_events.lock().unwrap().as_slice(),
            [IncidentEvent {
                kind: crate::domain::incident_event::IncidentEventKind::Created,
                ..
            }]
        ));
    }

    #[tokio::test]
    async fn non_manager_cannot_create_an_incident() {
        let requester_id = Uuid::new_v4();
        let team_id = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = CreateIncidentUseCase::new(teams, incidents.clone());

        let result = use_case
            .create_incident(CreateIncidentCommand {
                team_id,
                requester_id,
                title: "Primary DB latency".to_string(),
                description: String::new(),
                severity: Severity::High,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(incidents.saved.lock().unwrap().is_empty());
    }
}
