// Shared in-memory incident and timeline doubles.

use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::timeline::{ReactionRecord, TimelineEntry};
use crate::ports::{IncidentRepo, TimelineRepo};

#[derive(Default)]
pub struct MockIncidentRepo {
    pub incidents: Vec<Incident>,
    pub saved: Mutex<Vec<Incident>>,
    pub updated: Mutex<Vec<Incident>>,
    pub incident_events: Mutex<Vec<IncidentEvent>>,
    pub deleted: Mutex<Vec<Uuid>>,
    pub cleared: Mutex<Vec<(Uuid, Uuid)>>,
    pub reject_update: bool,
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

    pub fn rejecting_update(incident: Incident) -> Self {
        Self {
            incidents: vec![incident],
            reject_update: true,
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

    async fn update_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
        _expected_updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError> {
        if self.reject_update {
            return Err(DomainError::ConcurrentModification);
        }
        self.updated.lock().unwrap().push(incident.clone());
        self.incident_events.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<crate::ports::ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError> {
        let mut events: Vec<_> = self
            .incident_events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.incident_id == incident_id)
            .filter(|event| before.is_none_or(|cursor| (event.created_at, event.id) < cursor))
            .cloned()
            .collect();
        events.sort_by_key(|event| std::cmp::Reverse((event.created_at, event.id)));
        events.truncate(limit as usize);
        Ok(events)
    }

    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError> {
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
    pub attachment_members: Mutex<HashSet<Uuid>>,
    /// (entry_id, user_id, emoji) — the unique-per-tuple reaction store.
    pub reactions: Mutex<Vec<(Uuid, Uuid, String)>>,
}

impl MockTimelineRepo {
    pub fn allow_attachment_member(&self, user_id: Uuid) {
        self.attachment_members.lock().unwrap().insert(user_id);
    }
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
        before: Option<crate::ports::ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError> {
        let mut entries: Vec<_> = self
            .appended
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.incident_id == incident_id)
            .filter(|entry| before.is_none_or(|cursor| (entry.created_at, entry.id) < cursor))
            .cloned()
            .collect();
        entries.sort_by_key(|entry| std::cmp::Reverse((entry.created_at, entry.id)));
        entries.truncate(limit as usize);
        Ok(entries)
    }

    async fn find_entry_by_id(&self, entry_id: Uuid) -> Result<Option<TimelineEntry>, DomainError> {
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

    async fn find_attachment_for_member(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::domain::conversation::MessageAttachment>, DomainError> {
        if !self.attachment_members.lock().unwrap().contains(&user_id) {
            return Ok(None);
        }
        Ok(self
            .appended
            .lock()
            .unwrap()
            .iter()
            .flat_map(|entry| entry.attachments.iter())
            .find(|attachment| attachment.id == attachment_id)
            .cloned())
    }
}
