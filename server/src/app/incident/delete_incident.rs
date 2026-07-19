// --- server/src/app/incident/delete_incident.rs ---
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct DeleteIncidentCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
}

pub struct DeleteIncidentUseCase {
    incidents: Arc<dyn IncidentRepo>,
    teams: Arc<dyn TeamRepo>,
}

impl DeleteIncidentUseCase {
    pub fn new(incidents: Arc<dyn IncidentRepo>, teams: Arc<dyn TeamRepo>) -> Self {
        Self { incidents, teams }
    }

    /// Delete an incident. Enforces RBAC: only a Manager of the team can delete an incident.
    pub async fn delete_incident(&self, cmd: DeleteIncidentCommand) -> Result<(), DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        let role = self
            .teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::NotManager)?;

        if !derive_capabilities(role).can_delete_incident {
            return Err(DomainError::NotManager);
        }

        self.incidents.delete_incident(cmd.incident_id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::incident::{Incident, Severity};

    #[tokio::test]
    async fn manager_can_delete_incident() {
        let team_id = Uuid::new_v4();
        let manager_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Test", Severity::Low).unwrap();
        let incident_id = incident.id;

        let incidents = Arc::new(MockIncidentRepo::with_incident(incident));
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, manager_id, Role::Manager));
        let use_case = DeleteIncidentUseCase::new(incidents.clone(), teams);

        use_case
            .delete_incident(DeleteIncidentCommand {
                incident_id,
                requester_id: manager_id,
            })
            .await
            .unwrap();

        let deleted = incidents.deleted.lock().unwrap();
        assert_eq!(deleted.as_slice(), &[incident_id]);
    }

    #[tokio::test]
    async fn responder_cannot_delete_incident() {
        let team_id = Uuid::new_v4();
        let responder_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Test", Severity::Low).unwrap();

        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, responder_id, Role::Responder));
        let use_case = DeleteIncidentUseCase::new(incidents, teams);

        let result = use_case
            .delete_incident(DeleteIncidentCommand {
                incident_id: incident.id,
                requester_id: responder_id,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
    }

    #[tokio::test]
    async fn non_member_cannot_delete_incident() {
        let team_id = Uuid::new_v4();
        let stranger_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Test", Severity::Low).unwrap();

        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let teams = Arc::new(MockTeamRepo::default()); // Not a member
        let use_case = DeleteIncidentUseCase::new(incidents, teams);

        let result = use_case
            .delete_incident(DeleteIncidentCommand {
                incident_id: incident.id,
                requester_id: stranger_id,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
    }
}
