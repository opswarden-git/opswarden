// --- server/src/app/incident/list_incidents.rs ---
//
// Read use-case: the incidents of a team. RBAC: only a member of the team may
// list them (any role — Observer included). Returns the team's incidents.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct ListIncidentsCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListIncidentsResult {
    pub incidents: Vec<Incident>,
}

pub struct ListIncidentsUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl ListIncidentsUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    pub async fn list_incidents(
        &self,
        cmd: ListIncidentsCommand,
    ) -> Result<ListIncidentsResult, DomainError> {
        self.teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let incidents = self.incidents.list_incidents_for_team(cmd.team_id).await?;
        Ok(ListIncidentsResult { incidents })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::incident::Severity;
    use crate::domain::team::Role;

    #[tokio::test]
    async fn member_lists_team_incidents() {
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let incident = Incident::new(team, "DB latency", Severity::High).unwrap();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, user, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = ListIncidentsUseCase::new(teams, incidents);

        let result = use_case
            .list_incidents(ListIncidentsCommand {
                team_id: team,
                requester_id: user,
            })
            .await
            .unwrap();

        assert_eq!(result.incidents.len(), 1);
        assert_eq!(result.incidents[0].id, incident.id);
    }

    #[tokio::test]
    async fn non_member_is_forbidden() {
        let team = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default());
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = ListIncidentsUseCase::new(teams, incidents);

        let err = use_case
            .list_incidents(ListIncidentsCommand {
                team_id: team,
                requester_id: outsider,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }
}
