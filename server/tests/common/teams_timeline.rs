#[derive(Default)]
pub struct DummyTimelineRepo {
    entries: Mutex<Vec<TimelineEntry>>,
    reactions: Mutex<Vec<(Uuid, Uuid, String)>>,
}

#[allow(dead_code)]
impl DummyTimelineRepo {
    pub fn seed_entry(&self, entry: TimelineEntry) {
        self.entries.lock().unwrap().push(entry);
    }

    pub fn entries_for_incident(&self, incident_id: Uuid) -> Vec<TimelineEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.incident_id == incident_id)
            .cloned()
            .collect()
    }
}

#[async_trait]
impl TimelineRepo for DummyTimelineRepo {
    async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }

    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<opswarden_server::ports::ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError> {
        let mut entries: Vec<_> = self
            .entries
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
            .entries
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == entry_id)
            .cloned())
    }

    async fn update_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(slot) = entries.iter_mut().find(|e| e.id == entry.id) {
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
            .entries
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
        _user_id: Uuid,
    ) -> Result<Option<opswarden_server::domain::conversation::MessageAttachment>, DomainError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .flat_map(|entry| entry.attachments.iter())
            .find(|attachment| attachment.id == attachment_id)
            .cloned())
    }
}
