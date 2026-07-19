use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::timeline::TimelineEntry;
use crate::ports::{IncidentRepo, TeamRepo, TimelineRepo};

pub const DEFAULT_TIMELINE_LIMIT: u32 = 50;
pub const MAX_TIMELINE_LIMIT: u32 = 100;

pub struct ListTimelineEntriesCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub limit: Option<u32>,
}

/// Aggregated reactions for one emoji on one entry: how many users reacted and
/// whether the requesting user is among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: u64,
    pub reacted: bool,
}

/// A timeline entry enriched with its reaction summary, for the read view.
#[derive(Debug, PartialEq, Eq)]
pub struct TimelineEntryView {
    pub entry: TimelineEntry,
    pub reactions: Vec<ReactionSummary>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListTimelineEntriesResult {
    pub entries: Vec<TimelineEntryView>,
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

        // Aggregate reactions per (entry, emoji): a count and whether the
        // requester is one of the reactors.
        let reactions = self
            .timeline
            .list_reactions_for_incident(cmd.incident_id)
            .await?;
        let mut by_entry: HashMap<Uuid, HashMap<String, (u64, bool)>> = HashMap::new();
        for r in reactions {
            let slot = by_entry
                .entry(r.entry_id)
                .or_default()
                .entry(r.emoji)
                .or_insert((0, false));
            slot.0 += 1;
            if r.user_id == cmd.requester_id {
                slot.1 = true;
            }
        }

        let entries = entries
            .into_iter()
            .map(|entry| {
                let mut reactions: Vec<ReactionSummary> = by_entry
                    .get(&entry.id)
                    .map(|m| {
                        m.iter()
                            .map(|(emoji, (count, reacted))| ReactionSummary {
                                emoji: emoji.clone(),
                                count: *count,
                                reacted: *reacted,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                reactions.sort_by(|a, b| a.emoji.cmp(&b.emoji));
                TimelineEntryView { entry, reactions }
            })
            .collect();

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
        assert_eq!(result.entries[0].entry.content, "Investigating ingress");
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
        assert_eq!(result.entries[0].entry.content, "Entry number 2");
    }

    #[tokio::test]
    async fn list_aggregates_reaction_counts_and_reacted_flag() {
        let team_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Latency spike", Severity::High).unwrap();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester_id, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident.clone()));
        let timeline = Arc::new(MockTimelineRepo::default());
        let entry = TimelineEntry::new(incident.id, other_id, "Investigating").unwrap();
        timeline.append_entry(&entry).await.unwrap();
        // Two users react with 👍; only the requester also reacts with 🔥.
        timeline
            .add_reaction(entry.id, requester_id, "👍")
            .await
            .unwrap();
        timeline
            .add_reaction(entry.id, other_id, "👍")
            .await
            .unwrap();
        timeline
            .add_reaction(entry.id, requester_id, "🔥")
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

        let reactions = &result.entries[0].reactions;
        let thumbs = reactions.iter().find(|r| r.emoji == "👍").unwrap();
        assert_eq!(thumbs.count, 2);
        assert!(thumbs.reacted);
        let fire = reactions.iter().find(|r| r.emoji == "🔥").unwrap();
        assert_eq!(fire.count, 1);
        assert!(fire.reacted);
    }
}
