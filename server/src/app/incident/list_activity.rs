use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::timeline::TimelineEntry;
use crate::domain::user::UserSummary;
use crate::ports::{IncidentRepo, TeamRepo, TimelineRepo, UserRepo};

pub const DEFAULT_ACTIVITY_LIMIT: u32 = 50;
pub const MAX_ACTIVITY_LIMIT: u32 = 100;

/// Aggregated reactions for one emoji on one note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: u64,
    pub reacted: bool,
}

pub struct ListIncidentActivityCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncidentActivityItem {
    System {
        event: IncidentEvent,
        actor: Option<UserSummary>,
        subject: Option<UserSummary>,
    },
    Note {
        entry: TimelineEntry,
        author: Option<UserSummary>,
        reactions: Vec<ReactionSummary>,
    },
}

impl IncidentActivityItem {
    fn sort_key(&self) -> (DateTime<Utc>, Uuid) {
        match self {
            Self::System { event, .. } => (event.created_at, event.id),
            Self::Note { entry, .. } => (entry.created_at, entry.id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListIncidentActivityResult {
    pub items: Vec<IncidentActivityItem>,
}

pub struct ListIncidentActivityUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    timeline: Arc<dyn TimelineRepo>,
    users: Arc<dyn UserRepo>,
}

impl ListIncidentActivityUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        timeline: Arc<dyn TimelineRepo>,
        users: Arc<dyn UserRepo>,
    ) -> Self {
        Self {
            teams,
            incidents,
            timeline,
            users,
        }
    }

    pub async fn list(
        &self,
        cmd: ListIncidentActivityCommand,
    ) -> Result<ListIncidentActivityResult, DomainError> {
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
            .unwrap_or(DEFAULT_ACTIVITY_LIMIT)
            .clamp(1, MAX_ACTIVITY_LIMIT);
        // Fetch up to the requested amount from both sources, then merge and
        // truncate. This avoids one source starving the other while keeping a
        // hard upper bound until a cursor is justified by real history volume.
        let events = self
            .incidents
            .list_events_for_incident(cmd.incident_id, limit)
            .await?;
        let entries = self
            .timeline
            .list_entries_for_incident(cmd.incident_id, limit)
            .await?;
        let reaction_records = self
            .timeline
            .list_reactions_for_incident(cmd.incident_id)
            .await?;

        let mut reactions: HashMap<Uuid, HashMap<String, (u64, bool)>> = HashMap::new();
        for reaction in reaction_records {
            let summary = reactions
                .entry(reaction.entry_id)
                .or_default()
                .entry(reaction.emoji)
                .or_insert((0, false));
            summary.0 += 1;
            summary.1 |= reaction.user_id == cmd.requester_id;
        }

        let identity_ids: HashSet<Uuid> = events
            .iter()
            .filter_map(|event| event.actor_id)
            .chain(events.iter().filter_map(assigned_user_id))
            .chain(entries.iter().filter_map(|entry| entry.author_id))
            .collect();
        let mut identities: HashMap<Uuid, Option<UserSummary>> = HashMap::new();
        for user_id in identity_ids {
            let summary = self
                .users
                .find_by_id(user_id)
                .await?
                .as_ref()
                .map(UserSummary::from);
            identities.insert(user_id, summary);
        }

        let mut items: Vec<IncidentActivityItem> = events
            .into_iter()
            .map(|event| {
                let subject =
                    assigned_user_id(&event).and_then(|id| identities.get(&id).cloned().flatten());
                IncidentActivityItem::System {
                    actor: event
                        .actor_id
                        .and_then(|id| identities.get(&id).cloned().flatten()),
                    subject,
                    event,
                }
            })
            .chain(entries.into_iter().map(|entry| {
                let mut entry_reactions = reactions
                    .remove(&entry.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(emoji, (count, reacted))| ReactionSummary {
                        emoji,
                        count,
                        reacted,
                    })
                    .collect::<Vec<_>>();
                entry_reactions.sort_by(|a, b| a.emoji.cmp(&b.emoji));
                let author = entry
                    .author_id
                    .and_then(|id| identities.get(&id).cloned().flatten());
                IncidentActivityItem::Note {
                    entry,
                    author,
                    reactions: entry_reactions,
                }
            }))
            .collect();

        items.sort_by_key(|item| std::cmp::Reverse(item.sort_key()));
        items.truncate(limit as usize);
        Ok(ListIncidentActivityResult { items })
    }
}

fn assigned_user_id(event: &IncidentEvent) -> Option<Uuid> {
    event
        .data
        .get("assignee_id")
        .and_then(|value| value.as_str())
        .and_then(|value| Uuid::parse_str(value).ok())
}
