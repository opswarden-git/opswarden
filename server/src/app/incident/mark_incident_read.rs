use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::{IncidentRepo, TeamRepo};

pub struct MarkIncidentReadCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub read_through: DateTime<Utc>,
}

pub struct MarkIncidentReadUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl MarkIncidentReadUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    pub async fn mark(&self, command: MarkIncidentReadCommand) -> Result<(), DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(command.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;
        self.teams
            .find_member_role(incident.team_id, command.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        self.incidents
            .mark_incident_read(
                command.incident_id,
                command.requester_id,
                command.read_through,
            )
            .await
    }
}
