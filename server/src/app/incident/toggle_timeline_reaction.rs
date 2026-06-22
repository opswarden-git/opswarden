use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::timeline::validate_reaction_emoji;
use crate::ports::{EventPublisher, IncidentRepo, TeamRepo, TimelineRepo};

pub struct ToggleReactionCommand {
    pub incident_id: Uuid,
    pub entry_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ToggleReactionResult {
    pub emoji: String,
    /// `true` if the user now reacts with this emoji, `false` if it was removed.
    pub reacted: bool,
    pub count: u64,
}

pub struct ToggleReactionUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    timeline: Arc<dyn TimelineRepo>,
    events: Arc<dyn EventPublisher>,
}

impl ToggleReactionUseCase {
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

    /// Toggle the requester's reaction on an entry: add it if absent, remove it
    /// if present. Any team member (Observer included) may react.
    pub async fn toggle(
        &self,
        cmd: ToggleReactionCommand,
    ) -> Result<ToggleReactionResult, DomainError> {
        let incident = self
            .incidents
            .find_incident_by_id(cmd.incident_id)
            .await?
            .ok_or(DomainError::IncidentNotFound)?;

        self.teams
            .find_member_role(incident.team_id, cmd.user_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let entry = self
            .timeline
            .find_entry_by_id(cmd.entry_id)
            .await?
            .filter(|e| e.incident_id == incident.id)
            .ok_or(DomainError::IncidentNotFound)?;

        let emoji = validate_reaction_emoji(&cmd.emoji)?;

        // Add is idempotent (no duplicate); a non-new add means the user already
        // reacted, so this call toggles it off.
        let reacted = self
            .timeline
            .add_reaction(entry.id, cmd.user_id, &emoji)
            .await?;
        if !reacted {
            self.timeline
                .remove_reaction(entry.id, cmd.user_id, &emoji)
                .await?;
        }

        let count = self.timeline.count_reaction(entry.id, &emoji).await?;

        let event = if reacted {
            DomainEvent::ReactionAdded {
                team_id: incident.team_id,
                incident_id: entry.incident_id,
                entry_id: entry.id,
                emoji: emoji.clone(),
                user_id: cmd.user_id,
            }
        } else {
            DomainEvent::ReactionRemoved {
                team_id: incident.team_id,
                incident_id: entry.incident_id,
                entry_id: entry.id,
                emoji: emoji.clone(),
                user_id: cmd.user_id,
            }
        };
        self.events.publish(event).await;

        Ok(ToggleReactionResult {
            emoji,
            reacted,
            count,
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
        reactor_role: Role,
        reactor: Uuid,
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
        let teams = Arc::new(MockTeamRepo::default().with_member(team_id, reactor, reactor_role));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        let entry = TimelineEntry::new(incident.id, Uuid::new_v4(), "react to me").unwrap();
        timeline.append_entry(&entry).await.unwrap();
        let events = Arc::new(MockEventPublisher::default());
        (teams, incidents, timeline, events, incident, entry)
    }

    fn cmd(incident: &Incident, entry: &TimelineEntry, user: Uuid) -> ToggleReactionCommand {
        ToggleReactionCommand {
            incident_id: incident.id,
            entry_id: entry.id,
            user_id: user,
            emoji: "👍".to_string(),
        }
    }

    #[tokio::test]
    async fn observer_can_react_and_toggle() {
        let observer = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Observer, observer).await;
        let use_case = ToggleReactionUseCase::new(teams, incidents, timeline, events.clone());

        // First toggle adds.
        let added = use_case
            .toggle(cmd(&incident, &entry, observer))
            .await
            .unwrap();
        assert!(added.reacted);
        assert_eq!(added.count, 1);

        // Second toggle removes.
        let removed = use_case
            .toggle(cmd(&incident, &entry, observer))
            .await
            .unwrap();
        assert!(!removed.reacted);
        assert_eq!(removed.count, 0);

        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [
                DomainEvent::ReactionAdded { .. },
                DomainEvent::ReactionRemoved { .. }
            ]
        ));
    }

    #[tokio::test]
    async fn duplicate_reaction_does_not_increase_the_count() {
        let observer = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Observer, observer).await;
        let use_case = ToggleReactionUseCase::new(teams, incidents, timeline.clone(), events);

        use_case
            .toggle(cmd(&incident, &entry, observer))
            .await
            .unwrap();
        // The mock store holds exactly one reaction row for (entry, user, emoji).
        assert_eq!(timeline.reactions.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn non_member_cannot_react() {
        let stranger = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Observer, Uuid::new_v4()).await;
        let use_case = ToggleReactionUseCase::new(teams, incidents, timeline, events.clone());

        let result = use_case.toggle(cmd(&incident, &entry, stranger)).await;

        assert_eq!(result.unwrap_err(), DomainError::Forbidden);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn blank_emoji_is_rejected() {
        let observer = Uuid::new_v4();
        let (teams, incidents, timeline, events, incident, entry) =
            seed(Role::Observer, observer).await;
        let use_case = ToggleReactionUseCase::new(teams, incidents, timeline, events);

        let result = use_case
            .toggle(ToggleReactionCommand {
                incident_id: incident.id,
                entry_id: entry.id,
                user_id: observer,
                emoji: "   ".to_string(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidReaction);
    }
}
