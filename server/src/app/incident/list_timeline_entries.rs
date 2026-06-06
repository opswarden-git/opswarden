use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::{IncidentRepo, TeamRepo, TimelineRepo};

pub const DEFAULT_TIMELINE_LIMIT: u32 = 50;
pub const MAX_TIMELINE_LIMIT: u32 = 100;

pub struct ListTimelineEntriesCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub limit: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListTimelineEntriesResult {
    pub entries: Vec<crate::domain::timeline::TimelineEntry>,
}

pub struct ListTimelineEntriesUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    timeline: Arc<dyn TimelineRepo>,
}

impl ListTimelineEntriesUseCase {
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

    pub async fn list_entries(
        &self,
        cmd: ListTimelineEntriesCommand,
    ) -> Result<ListTimelineEntriesResult, DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        self.teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let limit = cmd
            .limit
            .unwrap_or(DEFAULT_TIMELINE_LIMIT)
            .clamp(1, MAX_TIMELINE_LIMIT);

        let entries = self
            .timeline
            .list_entries_for_incident(cmd.incident_id, limit)
            .await?;

        Ok(ListTimelineEntriesResult { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo, MockTimelineRepo};
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::Role;
    use crate::domain::timeline::TimelineEntry;
    use crate::ports::TimelineRepo;

    #[tokio::test]
    async fn observer_can_read_timeline_entries() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "API saturation", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        timeline
            .append_entry(
                &TimelineEntry::new(incident.id, requester_id, "Investigating ingress").unwrap(),
            )
            .await
            .unwrap();
        let use_case = ListTimelineEntriesUseCase::new(teams, incidents, timeline);

        let result = use_case
            .list_entries(ListTimelineEntriesCommand {
                incident_id: incident.id,
                requester_id,
                limit: None,
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].content, "Investigating ingress");
    }

    #[tokio::test]
    async fn timeline_limit_is_clamped() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "API saturation", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());

        for idx in 0..3 {
            timeline
                .append_entry(
                    &TimelineEntry::new(incident.id, requester_id, format!("Entry number {idx}"))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let use_case = ListTimelineEntriesUseCase::new(teams, incidents, timeline);
        let result = use_case
            .list_entries(ListTimelineEntriesCommand {
                incident_id: incident.id,
                requester_id,
                limit: Some(1),
            })
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].content, "Entry number 2");
    }
}
