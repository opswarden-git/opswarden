// --- server/src/app/incident/get_incident.rs ---
//
// Read use-case: a single incident by id. RBAC: only a member of the incident's
// team may read it. The timeline is a separate resource
// (`GET /api/incidents/{id}/timeline`), so this returns just the incident.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct GetIncidentCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GetIncidentResult {
    pub incident: Incident,
}

pub struct GetIncidentUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl GetIncidentUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    pub async fn get_incident(
        &self,
        cmd: GetIncidentCommand,
    ) -> Result<GetIncidentResult, DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        self.teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        Ok(GetIncidentResult { incident })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::incident::Severity;
    use crate::domain::team::Role;

    #[tokio::test]
    async fn member_reads_the_incident() {
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let incident = Incident::new(team, "Cache outage", Severity::Critical).unwrap();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, user, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = GetIncidentUseCase::new(teams, incidents);

        let result = use_case
            .get_incident(GetIncidentCommand {
                incident_id: incident.id,
                requester_id: user,
            })
            .await
            .unwrap();

        assert_eq!(result.incident.id, incident.id);
    }

    #[tokio::test]
    async fn non_member_is_forbidden() {
        let team = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let incident = Incident::new(team, "Cache outage", Severity::Critical).unwrap();
        let teams = Arc::new(MockTeamRepo::default());
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = GetIncidentUseCase::new(teams, incidents);

        let err = use_case
            .get_incident(GetIncidentCommand {
                incident_id: incident.id,
                requester_id: outsider,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }

    #[tokio::test]
    async fn unknown_incident_is_not_found() {
        let teams = Arc::new(MockTeamRepo::default());
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = GetIncidentUseCase::new(teams, incidents);

        let err = use_case
            .get_incident(GetIncidentCommand {
                incident_id: Uuid::new_v4(),
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::IncidentNotFound);
    }
}
