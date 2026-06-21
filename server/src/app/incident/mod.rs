pub mod add_timeline_entry;
pub mod assign_responder;
pub mod change_incident_status;
pub mod create_incident;
pub mod delete_incident;
pub mod get_incident;
pub mod list_incidents;
pub mod list_timeline_entries;

pub use add_timeline_entry::{
    AddTimelineEntryCommand, AddTimelineEntryResult, AddTimelineEntryUseCase,
};
pub use assign_responder::{AssignResponderCommand, AssignResponderResult, AssignResponderUseCase};
pub use change_incident_status::{
    ChangeIncidentStatusCommand, ChangeIncidentStatusResult, ChangeIncidentStatusUseCase,
};
pub use create_incident::{CreateIncidentCommand, CreateIncidentResult, CreateIncidentUseCase};
pub use delete_incident::{DeleteIncidentCommand, DeleteIncidentUseCase};
pub use get_incident::{GetIncidentCommand, GetIncidentResult, GetIncidentUseCase};
pub use list_incidents::{ListIncidentsCommand, ListIncidentsResult, ListIncidentsUseCase};
pub use list_timeline_entries::{
    ListTimelineEntriesCommand, ListTimelineEntriesResult, ListTimelineEntriesUseCase,
    DEFAULT_TIMELINE_LIMIT, MAX_TIMELINE_LIMIT,
};

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::event::DomainEvent;
    use crate::domain::incident::Incident;
    use crate::domain::team::{Role, TeamMemberView};
    use crate::domain::timeline::TimelineEntry;
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
                        },
                        *role,
                    )
                })
                .collect())
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
                })
                .collect())
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
        pub incident: Option<Incident>,
        pub saved: Mutex<Vec<Incident>>,
        pub updated: Mutex<Vec<Incident>>,
        pub deleted: Mutex<Vec<Uuid>>,
    }

    impl MockIncidentRepo {
        pub fn with_incident(incident: Incident) -> Self {
            Self {
                incident: Some(incident),
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

        async fn find_incident_by_id(
            &self,
            incident_id: Uuid,
        ) -> Result<Option<Incident>, DomainError> {
            Ok(self
                .incident
                .clone()
                .filter(|incident| incident.id == incident_id))
        }

        async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError> {
            self.updated.lock().unwrap().push(incident.clone());
            Ok(())
        }

        async fn list_incidents_for_team(
            &self,
            team_id: Uuid,
        ) -> Result<Vec<Incident>, DomainError> {
            Ok(self
                .incident
                .clone()
                .into_iter()
                .filter(|incident| incident.team_id == team_id)
                .collect())
        }

        async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError> {
            self.deleted.lock().unwrap().push(incident_id);
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MockTimelineRepo {
        pub appended: Mutex<Vec<TimelineEntry>>,
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
    }
}
