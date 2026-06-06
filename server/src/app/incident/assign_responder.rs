use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct AssignResponderCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub assignee_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AssignResponderResult {
    pub incident_id: Uuid,
    pub assignee_id: Uuid,
    pub changed: bool,
}

pub struct AssignResponderUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl AssignResponderUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    /// Assign a responder to an incident. Only a Manager may assign (403), and
    /// the assignee must be a team member able to act as Responder: an unknown
    /// member is 404, an Observer is 422.
    pub async fn assign(
        &self,
        cmd: AssignResponderCommand,
    ) -> Result<AssignResponderResult, DomainError> {
        let mut incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        let requester_role = self
            .teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !requester_role.can_act_as(Role::Manager) {
            return Err(DomainError::Forbidden);
        }

        let assignee_role = self
            .teams
            .find_member_role(incident.team_id, cmd.assignee_id)
            .await?
            .ok_or(DomainError::MemberNotFound)?;
        if !assignee_role.can_act_as(Role::Responder) {
            return Err(DomainError::AssigneeNotResponder);
        }

        let changed = incident.assign(cmd.assignee_id);
        if changed {
            self.incidents.update_incident(&incident).await?;
        }

        Ok(AssignResponderResult {
            incident_id: incident.id,
            assignee_id: cmd.assignee_id,
            changed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::incident::{Incident, Severity};

    #[tokio::test]
    async fn manager_can_assign_a_responder() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team_id, manager, Role::Manager)
                .with_member(team_id, responder, Role::Responder),
        );
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: incident.id,
                requester_id: manager,
                assignee_id: responder,
            })
            .await
            .unwrap();

        assert!(result.changed);
        assert_eq!(result.assignee_id, responder);
        assert_eq!(incidents.updated.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn assigning_same_responder_twice_is_idempotent() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let mut incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        incident.assign(responder);
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team_id, manager, Role::Manager)
                .with_member(team_id, responder, Role::Responder),
        );
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: incident.id,
                requester_id: manager,
                assignee_id: responder,
            })
            .await
            .unwrap();

        assert!(!result.changed);
        assert!(incidents.updated.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn non_manager_cannot_assign() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team_id, requester, Role::Responder)
                .with_member(team_id, responder, Role::Responder),
        );
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: incident.id,
                requester_id: requester,
                assignee_id: responder,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(incidents.updated.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cannot_assign_a_non_member() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, manager, Role::Manager));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: incident.id,
                requester_id: manager,
                assignee_id: stranger,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::MemberNotFound);
        assert!(incidents.updated.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cannot_assign_an_observer() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let incident = Incident::new(team_id, "Worker panic", Severity::High).unwrap();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team_id, manager, Role::Manager)
                .with_member(team_id, observer, Role::Observer),
        );
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: incident.id,
                requester_id: manager,
                assignee_id: observer,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::AssigneeNotResponder);
        assert!(incidents.updated.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn assigning_on_unknown_incident_returns_not_found() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, manager, Role::Manager));
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = AssignResponderUseCase::new(teams, incidents.clone());

        let result = use_case
            .assign(AssignResponderCommand {
                incident_id: Uuid::new_v4(),
                requester_id: manager,
                assignee_id: Uuid::new_v4(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::IncidentNotFound);
    }
}
