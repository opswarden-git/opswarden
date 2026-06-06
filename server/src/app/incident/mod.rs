pub mod add_timeline_entry;
pub mod assign_responder;
pub mod change_incident_status;
pub mod create_incident;
pub mod list_timeline_entries;

pub use add_timeline_entry::{
    AddTimelineEntryCommand, AddTimelineEntryResult, AddTimelineEntryUseCase,
};
pub use assign_responder::{AssignResponderCommand, AssignResponderResult, AssignResponderUseCase};
pub use change_incident_status::{
    ChangeIncidentStatusCommand, ChangeIncidentStatusResult, ChangeIncidentStatusUseCase,
};
pub use create_incident::{CreateIncidentCommand, CreateIncidentResult, CreateIncidentUseCase};
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
    use crate::domain::team::Role;
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
