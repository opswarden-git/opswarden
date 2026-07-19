pub mod add_timeline_entry;
pub mod assign_responder;
pub mod change_incident_status;
pub mod create_incident;
pub mod delete_incident;
pub mod edit_timeline_entry;
pub mod get_incident;
pub mod list_activity;
pub mod list_incidents;
pub mod list_timeline_entries;
pub mod toggle_timeline_reaction;

pub use add_timeline_entry::{
    AddTimelineEntryCommand, AddTimelineEntryResult, AddTimelineEntryUseCase,
};
pub use assign_responder::{AssignResponderCommand, AssignResponderResult, AssignResponderUseCase};
pub use change_incident_status::{
    ChangeIncidentStatusCommand, ChangeIncidentStatusResult, ChangeIncidentStatusUseCase,
};
pub use create_incident::{CreateIncidentCommand, CreateIncidentResult, CreateIncidentUseCase};
pub use delete_incident::{DeleteIncidentCommand, DeleteIncidentUseCase};
pub use edit_timeline_entry::{
    EditTimelineEntryCommand, EditTimelineEntryResult, EditTimelineEntryUseCase,
};
pub use get_incident::{GetIncidentCommand, GetIncidentResult, GetIncidentUseCase};
pub use list_activity::{
    IncidentActivityItem, ListIncidentActivityCommand, ListIncidentActivityResult,
    ListIncidentActivityUseCase, DEFAULT_ACTIVITY_LIMIT, MAX_ACTIVITY_LIMIT,
};
pub use list_incidents::{
    IncidentAssigneeFilter, IncidentCounts, IncidentListItem, IncidentSort, ListIncidentsCommand,
    ListIncidentsResult, ListIncidentsUseCase,
};
pub use list_timeline_entries::{
    ListTimelineEntriesCommand, ListTimelineEntriesResult, ListTimelineEntriesUseCase,
    ReactionSummary, TimelineEntryView, DEFAULT_TIMELINE_LIMIT, MAX_TIMELINE_LIMIT,
};
pub use toggle_timeline_reaction::{
    ToggleReactionCommand, ToggleReactionResult, ToggleReactionUseCase,
};

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::event::DomainEvent;
    use crate::domain::incident::Incident;
    use crate::domain::incident_event::IncidentEvent;
    use crate::domain::team::{Role, TeamBanView, TeamDirectoryItem, TeamMemberView};
    use crate::domain::timeline::{ReactionRecord, TimelineEntry};
    use crate::ports::{EventPublisher, IncidentRepo, TeamRepo, TimelineRepo};

    #[derive(Default)]
    pub struct MockTeamRepo {
        pub roles: HashMap<(Uuid, Uuid), Role>,
    }

    impl MockTeamRepo {
        pub fn with_member(mut self, team_id: Uuid, user_id: Uuid, role: Role) -> Self {
            self.roles.insert((team_id, user_id), role);
            self
        }
    }

    #[async_trait]
    impl TeamRepo for MockTeamRepo {
        async fn save_team(&self, _team: &crate::domain::team::Team) -> Result<(), DomainError> {
            Ok(())
        }

        async fn find_by_invitation_code(
            &self,
            _code: &str,
        ) -> Result<Option<crate::domain::team::Team>, DomainError> {
            Ok(None)
        }

        async fn find_member_role(
            &self,
            team_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<Role>, DomainError> {
            Ok(self.roles.get(&(team_id, user_id)).copied())
        }

        async fn add_member(
            &self,
            _team_id: Uuid,
            _user_id: Uuid,
            _role: Role,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn transfer_manager(
            &self,
            _team_id: Uuid,
            _old_manager: Uuid,
            _new_manager: Uuid,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
            Ok(self
                .roles
                .keys()
                .filter(|(_, u)| *u == user_id)
                .map(|(t, _)| *t)
                .collect())
        }

        async fn list_teams_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<(crate::domain::team::Team, Role)>, DomainError> {
            use crate::domain::team::{InvitationCode, Team};
            Ok(self
                .roles
                .iter()
                .filter(|((_, u), _)| *u == user_id)
                .map(|((t, _), role)| {
                    (
                        Team {
                            id: *t,
                            name: format!("team-{t}"),
                            invitation_code: InvitationCode::from_existing("OPS-TEST00"),
                            created_at: Utc::now(),
                        },
                        *role,
                    )
                })
                .collect())
        }

        async fn list_team_directory_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<TeamDirectoryItem>, DomainError> {
            Ok(self
                .list_teams_for_user(user_id)
                .await?
                .into_iter()
                .map(|(team, role)| TeamDirectoryItem {
                    team,
                    role,
                    member_count: self.roles.len() as u64,
                    active_incident_count: 0,
                    active_release_count: 0,
                    blocked_release_count: 0,
                })
                .collect())
        }

        async fn find_team_by_id(
            &self,
            _team_id: Uuid,
        ) -> Result<Option<crate::domain::team::Team>, DomainError> {
            Ok(None)
        }

        async fn delete_team(&self, _team_id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn remove_member(&self, _team_id: Uuid, _user_id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError> {
            Ok(self.roles.keys().filter(|(t, _)| *t == team_id).count() as u64)
        }

        async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError> {
            Ok(self
                .roles
                .iter()
                .filter(|((t, _), _)| *t == team_id)
                .map(|((_, user_id), role)| TeamMemberView {
                    user_id: *user_id,
                    email: format!("user-{user_id}@test.local"),
                    role: *role,
                    joined_at: Utc::now(),
                })
                .collect())
        }

        async fn set_member_role(
            &self,
            _team_id: Uuid,
            _user_id: Uuid,
            _role: Role,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn add_ban(&self, _ban: &crate::domain::team::TeamBan) -> Result<(), DomainError> {
            Ok(())
        }

        async fn find_ban(
            &self,
            _team_id: Uuid,
            _user_id: Uuid,
        ) -> Result<Option<crate::domain::team::TeamBan>, DomainError> {
            Ok(None)
        }

        async fn list_bans(&self, _team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
            Ok(Vec::new())
        }

        async fn remove_ban(&self, _team_id: Uuid, _user_id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MockEventPublisher {
        pub published: Mutex<Vec<DomainEvent>>,
    }

    #[async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, event: DomainEvent) {
            self.published.lock().unwrap().push(event);
        }
    }

    #[derive(Default)]
    pub struct MockIncidentRepo {
        pub incidents: Vec<Incident>,
        pub saved: Mutex<Vec<Incident>>,
        pub updated: Mutex<Vec<Incident>>,
        pub incident_events: Mutex<Vec<IncidentEvent>>,
        pub deleted: Mutex<Vec<Uuid>>,
        pub cleared: Mutex<Vec<(Uuid, Uuid)>>,
    }

    impl MockIncidentRepo {
        pub fn with_incident(incident: Incident) -> Self {
            Self {
                incidents: vec![incident],
                ..Self::default()
            }
        }

        pub fn with_incidents(incidents: Vec<Incident>) -> Self {
            Self {
                incidents,
                ..Self::default()
            }
        }
    }

    #[async_trait]
    impl IncidentRepo for MockIncidentRepo {
        async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError> {
            self.saved.lock().unwrap().push(incident.clone());
            Ok(())
        }

        async fn save_incident_with_event(
            &self,
            incident: &Incident,
            event: &IncidentEvent,
        ) -> Result<(), DomainError> {
            self.saved.lock().unwrap().push(incident.clone());
            self.incident_events.lock().unwrap().push(event.clone());
            Ok(())
        }

        async fn find_incident_by_id(
            &self,
            incident_id: Uuid,
        ) -> Result<Option<Incident>, DomainError> {
            Ok(self
                .incidents
                .iter()
                .find(|incident| incident.id == incident_id)
                .cloned())
        }

        async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError> {
            self.updated.lock().unwrap().push(incident.clone());
            Ok(())
        }

        async fn update_incident_with_event(
            &self,
            incident: &Incident,
            event: &IncidentEvent,
        ) -> Result<(), DomainError> {
            self.updated.lock().unwrap().push(incident.clone());
            self.incident_events.lock().unwrap().push(event.clone());
            Ok(())
        }

        async fn list_events_for_incident(
            &self,
            incident_id: Uuid,
            limit: u32,
        ) -> Result<Vec<IncidentEvent>, DomainError> {
            let mut events: Vec<_> = self
                .incident_events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.incident_id == incident_id)
                .cloned()
                .collect();
            events.sort_by_key(|event| std::cmp::Reverse((event.created_at, event.id)));
            events.truncate(limit as usize);
            Ok(events)
        }

        async fn list_incidents_for_team(
            &self,
            team_id: Uuid,
        ) -> Result<Vec<Incident>, DomainError> {
            Ok(self
                .incidents
                .iter()
                .filter(|incident| incident.team_id == team_id)
                .cloned()
                .collect())
        }

        async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError> {
            self.deleted.lock().unwrap().push(incident_id);
            Ok(())
        }

        async fn clear_assignee_for_member(
            &self,
            team_id: Uuid,
            user_id: Uuid,
        ) -> Result<(), DomainError> {
            self.cleared.lock().unwrap().push((team_id, user_id));
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MockTimelineRepo {
        pub appended: Mutex<Vec<TimelineEntry>>,
        /// (entry_id, user_id, emoji) — the unique-per-tuple reaction store.
        pub reactions: Mutex<Vec<(Uuid, Uuid, String)>>,
    }

    #[async_trait]
    impl TimelineRepo for MockTimelineRepo {
        async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
            self.appended.lock().unwrap().push(entry.clone());
            Ok(())
        }

        async fn list_entries_for_incident(
            &self,
            incident_id: Uuid,
            limit: u32,
        ) -> Result<Vec<TimelineEntry>, DomainError> {
            let mut entries: Vec<_> = self
                .appended
                .lock()
                .unwrap()
                .iter()
                .filter(|entry| entry.incident_id == incident_id)
                .cloned()
                .collect();
            entries.reverse();
            entries.truncate(limit as usize);
            Ok(entries)
        }

        async fn find_entry_by_id(
            &self,
            entry_id: Uuid,
        ) -> Result<Option<TimelineEntry>, DomainError> {
            Ok(self
                .appended
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == entry_id)
                .cloned())
        }

        async fn update_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
            let mut appended = self.appended.lock().unwrap();
            if let Some(slot) = appended.iter_mut().find(|e| e.id == entry.id) {
                *slot = entry.clone();
            }
            Ok(())
        }

        async fn add_reaction(
            &self,
            entry_id: Uuid,
            user_id: Uuid,
            emoji: &str,
        ) -> Result<bool, DomainError> {
            let mut reactions = self.reactions.lock().unwrap();
            let key = (entry_id, user_id, emoji.to_string());
            if reactions.contains(&key) {
                return Ok(false);
            }
            reactions.push(key);
            Ok(true)
        }

        async fn remove_reaction(
            &self,
            entry_id: Uuid,
            user_id: Uuid,
            emoji: &str,
        ) -> Result<(), DomainError> {
            self.reactions
                .lock()
                .unwrap()
                .retain(|(e, u, em)| !(*e == entry_id && *u == user_id && em == emoji));
            Ok(())
        }

        async fn count_reaction(&self, entry_id: Uuid, emoji: &str) -> Result<u64, DomainError> {
            Ok(self
                .reactions
                .lock()
                .unwrap()
                .iter()
                .filter(|(e, _, em)| *e == entry_id && em == emoji)
                .count() as u64)
        }

        async fn list_reactions_for_incident(
            &self,
            incident_id: Uuid,
        ) -> Result<Vec<ReactionRecord>, DomainError> {
            let entry_ids: Vec<Uuid> = self
                .appended
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.incident_id == incident_id)
                .map(|e| e.id)
                .collect();
            Ok(self
                .reactions
                .lock()
                .unwrap()
                .iter()
                .filter(|(e, _, _)| entry_ids.contains(e))
                .map(|(entry_id, user_id, emoji)| ReactionRecord {
                    entry_id: *entry_id,
                    user_id: *user_id,
                    emoji: emoji.clone(),
                })
                .collect())
        }
    }
}
