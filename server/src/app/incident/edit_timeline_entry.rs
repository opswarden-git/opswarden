use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::ports::{EventPublisher, IncidentRepo, TeamRepo, TimelineRepo};

pub struct EditTimelineEntryCommand {
    pub incident_id: Uuid,
    pub entry_id: Uuid,
    pub requester_id: Uuid,
    pub content: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EditTimelineEntryResult {
    pub entry_id: Uuid,
    pub incident_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

pub struct EditTimelineEntryUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    timeline: Arc<dyn TimelineRepo>,
    events: Arc<dyn EventPublisher>,
}

impl EditTimelineEntryUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        timeline: Arc<dyn TimelineRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            incidents,
            timeline,
            events,
        }
    }

    /// Edit a timeline entry's content. The requester must belong to the
    /// incident's team (access) and be the entry's author (author-only edit).
    pub async fn edit(
        &self,
        cmd: EditTimelineEntryCommand,
    ) -> Result<EditTimelineEntryResult, DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        self.teams
            .find_member_role(incident.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let mut entry = self
            .timeline
            .find_entry_by_id(cmd.entry_id)
            .await?
            .filter(|e| e.incident_id == incident.id)
            .ok_or(DomainError::IncidentNotFound)?;

        if entry.author_id != Some(cmd.requester_id) {
            return Err(DomainError::Forbidden);
        }

        entry.edit(cmd.content)?;
        self.timeline.update_entry(&entry).await?;

        let edited_at = entry.edited_at.unwrap_or_else(Utc::now);
        self.events
            .publish(DomainEvent::TimelineEntryEdited {
                team_id: incident.team_id,
                incident_id: entry.incident_id,
                entry_id: entry.id,
                content: entry.content.clone(),
                edited_at,
            })
            .await;

        Ok(EditTimelineEntryResult {
            entry_id: entry.id,
            incident_id: entry.incident_id,
            author_id: cmd.requester_id,
            content: entry.content,
            created_at: entry.created_at,
            edited_at: entry.edited_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{
        MockEventPublisher, MockIncidentRepo, MockTeamRepo, MockTimelineRepo,
    };
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::Role;
    use crate::domain::timeline::TimelineEntry;

    async fn seed(
        author_role: Role,
        author_id: Uuid,
    ) -> (
        Arc<MockTeamRepo>,
        Arc<MockIncidentRepo>,
        Arc<MockTimelineRepo>,
        Arc<MockEventPublisher>,
        Incident,
        TimelineEntry,
    ) {
        let team_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Cache outage", Severity::High).unwrap();
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, author_id, author_role));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        let entry = TimelineEntry::new(incident.id, author_id, "original").unwrap();
        timeline.append_entry(&entry).await.unwrap();
        let events = Arc::new(MockEventPublisher::default());
        (teams, incidents, timeline, events, incident, entry)
    }

    #[tokio::test]
    async fn author_can_edit_their_entry() {
        let author = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Responder, author).await;
        let use_case =
            EditTimelineEntryUseCase::new(teams, incidents, timeline.clone(), events.clone());

        let result = use_case
            .edit(EditTimelineEntryCommand {
                incident_id: incident.id,
                entry_id: entry.id,
                requester_id: author,
                content: "updated content".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(result.content, "updated content");
        assert!(result.edited_at.is_some());
        assert_eq!(result.created_at, entry.created_at);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::TimelineEntryEdited { .. }]
        ));
    }

    #[tokio::test]
    async fn non_author_member_cannot_edit() {
        let author = Uuid::new_v4();
        let other = Uuid::new_v4();
        let (_teams, incidents, timeline, events, incident, entry) =
            seed(Role::Responder, author).await;
        // `other` is also a team member but not the author, so use a team repo
        // that knows both.
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(incident.team_id, author, Role::Responder)
                .with_member(incident.team_id, other, Role::Manager),
        );
        let use_case =
            EditTimelineEntryUseCase::new(teams, incidents, timeline.clone(), events.clone());

        let result = use_case
            .edit(EditTimelineEntryCommand {
                incident_id: incident.id,
                entry_id: entry.id,
                requester_id: other,
                content: "hijack".to_string(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn non_member_cannot_edit() {
        let author = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Responder, author).await;
        let use_case = EditTimelineEntryUseCase::new(teams, incidents, timeline, events.clone());

        let result = use_case
            .edit(EditTimelineEntryCommand {
                incident_id: incident.id,
                entry_id: entry.id,
                requester_id: stranger,
                content: "stranger edit".to_string(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn blank_edit_is_rejected() {
        let author = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Responder, author).await;
        let use_case = EditTimelineEntryUseCase::new(teams, incidents, timeline, events);

        let result = use_case
            .edit(EditTimelineEntryCommand {
                incident_id: incident.id,
                entry_id: entry.id,
                requester_id: author,
                content: "   ".to_string(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidTimelineEntry);
    }
}
