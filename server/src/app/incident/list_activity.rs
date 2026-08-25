use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::conversation::MessageReactionSummary;
use crate::domain::error::DomainError;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::timeline::TimelineEntry;
use crate::domain::user::UserSummary;
use crate::ports::{ActivityCursor, IncidentRepo, TeamRepo, TimelineRepo, UserRepo};

pub const DEFAULT_ACTIVITY_LIMIT: u32 = 50;
pub const MAX_ACTIVITY_LIMIT: u32 = 100;

pub type ReactionSummary = MessageReactionSummary;

pub struct ListIncidentActivityCommand {
    pub incident_id: Uuid,
    pub requester_id: Uuid,
    pub limit: Option<u32>,
    /// Walk further back: only activity strictly older than this cursor.
    pub before: Option<ActivityCursor>,
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
    fn sort_key(&self) -> ActivityCursor {
        match self {
            Self::System { event, .. } => (event.created_at, event.id),
            Self::Note { entry, .. } => (entry.created_at, entry.id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListIncidentActivityResult {
    pub team_id: Uuid,
    pub items: Vec<IncidentActivityItem>,
    /// `Some` while older activity remains; feed it back as `before`.
    pub next_cursor: Option<ActivityCursor>,
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
        // One extra row from each source: after merging, holding more than the
        // page size is exactly the signal that older activity remains. Asking
        // each source for a full page keeps a chatty timeline from starving the
        // event log, or the reverse.
        let probe = limit + 1;
        let events = self
            .incidents
            .list_events_for_incident(cmd.incident_id, cmd.before, probe)
            .await?;
        let entries = self
            .timeline
            .list_entries_for_incident(cmd.incident_id, cmd.before, probe)
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
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        // The cursor is the oldest item actually handed out, so the next page
        // resumes exactly where this one stopped.
        let next_cursor = has_more
            .then(|| items.last().map(IncidentActivityItem::sort_key))
            .flatten();
        Ok(ListIncidentActivityResult {
            team_id: incident.team_id,
            items,
            next_cursor,
        })
    }
}

fn assigned_user_id(event: &IncidentEvent) -> Option<Uuid> {
    event
        .data
        .get("assignee_id")
        .and_then(|value| value.as_str())
        .and_then(|value| Uuid::parse_str(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo, MockTimelineRepo};
    use crate::app::private_message::tests::MockUserRepo;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::incident_event::IncidentEvent;
    use crate::domain::team::Role;
    use chrono::{Duration, Utc};

    /// A war room with interleaved system events and human notes, each a minute
    /// apart so the merge order is unambiguous.
    fn seeded() -> (Incident, Uuid, ListIncidentActivityUseCase) {
        let team_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let incident = Incident::new(team_id, "Cache outage", Severity::Critical).unwrap();
        let start = Utc::now() - Duration::minutes(60);

        let incidents = MockIncidentRepo::with_incident(incident.clone());
        let timeline = MockTimelineRepo::default();
        for step in 0..6 {
            let mut event = IncidentEvent::created(&incident, Some(member));
            event.created_at = start + Duration::minutes(step * 2);
            incidents.incident_events.lock().unwrap().push(event);

            let mut entry =
                TimelineEntry::new(incident.id, member, format!("note {step}")).unwrap();
            entry.created_at = start + Duration::minutes(step * 2 + 1);
            timeline.appended.lock().unwrap().push(entry);
        }

        let teams = MockTeamRepo::default().with_member(team_id, member, Role::Responder);
        let use_case = ListIncidentActivityUseCase::new(
            Arc::new(teams),
            Arc::new(incidents),
            Arc::new(timeline),
            Arc::new(MockUserRepo::default().with_user(member)),
        );
        (incident, member, use_case)
    }

    async fn page(
        use_case: &ListIncidentActivityUseCase,
        incident_id: Uuid,
        requester_id: Uuid,
        before: Option<ActivityCursor>,
    ) -> ListIncidentActivityResult {
        use_case
            .list(ListIncidentActivityCommand {
                incident_id,
                requester_id,
                limit: Some(5),
                before,
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_full_page_reports_that_older_activity_remains() {
        let (incident, member, use_case) = seeded();

        let first = page(&use_case, incident.id, member, None).await;

        assert_eq!(first.items.len(), 5);
        assert!(first.next_cursor.is_some());
        // Newest first, and both sources are represented in the same page.
        let keys: Vec<_> = first.items.iter().map(|item| item.sort_key()).collect();
        let mut sorted = keys.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(keys, sorted);
    }

    #[tokio::test]
    async fn following_the_cursor_yields_strictly_older_activity_without_overlap() {
        let (incident, member, use_case) = seeded();

        let first = page(&use_case, incident.id, member, None).await;
        let second = page(&use_case, incident.id, member, first.next_cursor).await;

        let first_keys: Vec<_> = first.items.iter().map(|item| item.sort_key()).collect();
        let second_keys: Vec<_> = second.items.iter().map(|item| item.sort_key()).collect();
        assert!(second_keys.iter().all(|key| !first_keys.contains(key)));
        assert!(second_keys
            .iter()
            .all(|key| *key < *first_keys.last().unwrap()));
    }

    #[tokio::test]
    async fn walking_to_the_end_reaches_every_item_exactly_once_and_stops() {
        let (incident, member, use_case) = seeded();

        let mut seen: Vec<ActivityCursor> = Vec::new();
        let mut cursor = None;
        loop {
            let result = page(&use_case, incident.id, member, cursor).await;
            seen.extend(result.items.iter().map(|item| item.sort_key()));
            match result.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        // Six events and six notes, each returned once.
        assert_eq!(seen.len(), 12);
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 12);
    }
}
