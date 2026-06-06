use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::domain::timeline::TimelineEntry;
use crate::ports::{IncidentRepo, TeamRepo, TimelineRepo};

pub struct AddTimelineEntryCommand {
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AddTimelineEntryResult {
    pub entry_id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
}

pub struct AddTimelineEntryUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    timeline: Arc<dyn TimelineRepo>,
}

impl AddTimelineEntryUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        timeline: Arc<dyn TimelineRepo>,
    ) -> Self {
        Self {
            teams,
            incidents,
            timeline,
        }
    }

    pub async fn add_timeline_entry(
        &self,
        cmd: AddTimelineEntryCommand,
    ) -> Result<AddTimelineEntryResult, DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        let role = self
            .teams
            .find_member_role(incident.team_id, cmd.author_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        if !role.can_act_as(Role::Responder) {
            return Err(DomainError::Forbidden);
        }

        let entry = TimelineEntry::new(cmd.incident_id, cmd.author_id, cmd.content)?;
        self.timeline.append_entry(&entry).await?;

        Ok(AddTimelineEntryResult {
            entry_id: entry.id,
            incident_id: entry.incident_id,
            author_id: entry.author_id,
            content: entry.content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo, MockTimelineRepo};
    use crate::domain::incident::{Incident, Severity};

    #[tokio::test]
    async fn responder_can_post_a_timeline_entry() {
        let team_id = Uuid::new_v4();
        let author_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Cache outage", Severity::Critical).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, author_id, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        let use_case = AddTimelineEntryUseCase::new(teams, incidents, timeline.clone());

        let result = use_case
            .add_timeline_entry(AddTimelineEntryCommand {
                incident_id: incident.id,
                author_id,
                content: "Investigating upstream saturation".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(result.incident_id, incident.id);
        assert_eq!(timeline.appended.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn observer_cannot_post_a_timeline_entry() {
        let team_id = Uuid::new_v4();
        let author_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Cache outage", Severity::Critical).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, author_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        let use_case = AddTimelineEntryUseCase::new(teams, incidents, timeline.clone());

        let result = use_case
            .add_timeline_entry(AddTimelineEntryCommand {
                incident_id: incident.id,
                author_id,
                content: "I should not be able to post".to_string(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(timeline.appended.lock().unwrap().is_empty());
    }
}
