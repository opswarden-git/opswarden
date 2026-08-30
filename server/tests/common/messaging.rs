#[allow(dead_code)]
#[derive(Default)]
pub struct DummyPrivateMessageRepo {
    messages: Mutex<Vec<PrivateMessage>>,
    reactions: Mutex<HashSet<(Uuid, Uuid, String)>>,
    reads: Mutex<HashMap<(Uuid, Uuid), DateTime<Utc>>>,
}

#[allow(dead_code)]
impl DummyPrivateMessageRepo {
    pub fn seed(&self, message: PrivateMessage) {
        self.messages.lock().unwrap().push(message);
    }

    pub fn all(&self) -> Vec<PrivateMessage> {
        self.messages.lock().unwrap().clone()
    }
}

#[async_trait]
impl PrivateMessageRepo for DummyPrivateMessageRepo {
    async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError> {
        self.messages.lock().unwrap().push(message.clone());
        Ok(())
    }

    async fn list_conversation(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        before: Option<(DateTime<Utc>, Uuid)>,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError> {
        let reactions = self.reactions.lock().unwrap();
        let mut msgs: Vec<PrivateMessage> = self
            .messages
            .lock()
            .unwrap()
            .iter()
            .filter(|m| {
                ((m.sender_id == viewer_id && m.recipient_id == peer_id)
                    || (m.sender_id == peer_id && m.recipient_id == viewer_id))
                    && before.is_none_or(|cursor| (m.created_at, m.id) < cursor)
            })
            .cloned()
            .collect();
        for message in &mut msgs {
            let mut by_emoji: HashMap<String, (u64, bool)> = HashMap::new();
            for (_, user_id, emoji) in reactions.iter().filter(|(id, _, _)| *id == message.id) {
                let summary = by_emoji.entry(emoji.clone()).or_default();
                summary.0 += 1;
                summary.1 |= *user_id == viewer_id;
            }
            message.reactions = by_emoji
                .into_iter()
                .map(|(emoji, (count, reacted))| PrivateMessageReaction {
                    emoji,
                    count,
                    reacted,
                })
                .collect();
        }
        msgs.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        msgs.truncate(limit as usize);
        Ok(msgs)
    }

    async fn find_participants(
        &self,
        message_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, DomainError> {
        Ok(self
            .messages
            .lock()
            .unwrap()
            .iter()
            .find(|message| message.id == message_id)
            .map(|message| (message.sender_id, message.recipient_id)))
    }

    async fn update_content(
        &self,
        message_id: Uuid,
        content: &str,
        edited_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if let Some(message) = self
            .messages
            .lock()
            .unwrap()
            .iter_mut()
            .find(|message| message.id == message_id)
        {
            message.content = content.to_string();
            message.edited_at = Some(edited_at);
        }
        Ok(())
    }

    async fn toggle_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, DomainError> {
        let key = (message_id, user_id, emoji.to_string());
        let mut reactions = self.reactions.lock().unwrap();
        if reactions.remove(&key) {
            Ok(false)
        } else {
            reactions.insert(key);
            Ok(true)
        }
    }

    async fn find_attachment_for_participant(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PrivateMessageAttachment>, DomainError> {
        Ok(self
            .messages
            .lock()
            .unwrap()
            .iter()
            .filter(|message| message.sender_id == user_id || message.recipient_id == user_id)
            .flat_map(|message| message.attachments.iter())
            .find(|attachment| attachment.id == attachment_id)
            .cloned())
    }

    async fn mark_read(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        read_through: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut reads = self.reads.lock().unwrap();
        let entry = reads.entry((viewer_id, peer_id)).or_insert(read_through);
        if read_through > *entry {
            *entry = read_through;
        }
        Ok(())
    }

    async fn list_unread_peer_ids(&self, viewer_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let messages = self.messages.lock().unwrap();
        let reads = self.reads.lock().unwrap();
        let mut unread_senders = HashSet::new();

        for msg in messages.iter() {
            if msg.recipient_id == viewer_id {
                let read_through = reads.get(&(viewer_id, msg.sender_id));
                if read_through.is_none() || msg.created_at > *read_through.unwrap() {
                    unread_senders.insert(msg.sender_id);
                }
            }
        }
        Ok(unread_senders.into_iter().collect())
    }
}

/// In-memory release repo. Crucially its `count_active_linked_incidents` reads
/// live incident statuses from the shared `DummyIncidentRepo`, so resolving an
/// incident really unblocks a linked release in HTTP tests.
pub struct DummyReleaseRepo {
    releases: Mutex<HashMap<Uuid, Release>>,
    links: Mutex<Vec<(Uuid, Uuid)>>,
    incidents: Arc<DummyIncidentRepo>,
}

#[allow(dead_code)]
impl DummyReleaseRepo {
    pub fn new(incidents: Arc<DummyIncidentRepo>) -> Self {
        Self {
            releases: Mutex::new(HashMap::new()),
            links: Mutex::new(Vec::new()),
            incidents,
        }
    }
}

#[async_trait]
impl ReleaseRepo for DummyReleaseRepo {
    async fn save_release(&self, release: &Release) -> Result<(), DomainError> {
        self.releases
            .lock()
            .unwrap()
            .insert(release.id, release.clone());
        Ok(())
    }

    async fn create_release(
        &self,
        release: &Release,
        _delivery_id: &str,
        _event: &opswarden_server::domain::automation::ExternalEvent,
    ) -> Result<(), DomainError> {
        self.save_release(release).await
    }

    async fn create_blocking_incident(
        &self,
        release_id: Uuid,
        _expected_updated_at: chrono::DateTime<chrono::Utc>,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        self.incidents
            .save_incident_with_event(incident, event)
            .await?;
        self.link_incident(release_id, incident.id).await
    }

    async fn find_release_by_id(&self, release_id: Uuid) -> Result<Option<Release>, DomainError> {
        Ok(self.releases.lock().unwrap().get(&release_id).cloned())
    }

    async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError> {
        Ok(self
            .releases
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.team_id == team_id)
            .cloned()
            .collect())
    }

    async fn update_release(
        &self,
        release: &Release,
        _expected_updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError> {
        self.releases
            .lock()
            .unwrap()
            .insert(release.id, release.clone());
        Ok(())
    }

    async fn update_release_with_incident_events(
        &self,
        release: &Release,
        expected_updated_at: chrono::DateTime<chrono::Utc>,
        events: &[IncidentEvent],
    ) -> Result<(), DomainError> {
        self.update_release(release, expected_updated_at).await?;
        self.incidents.record_events(events);
        Ok(())
    }

    async fn link_incident(&self, release_id: Uuid, incident_id: Uuid) -> Result<(), DomainError> {
        let mut links = self.links.lock().unwrap();
        if !links.contains(&(release_id, incident_id)) {
            links.push((release_id, incident_id));
        }
        Ok(())
    }

    async fn unlink_incident(
        &self,
        release_id: Uuid,
        incident_id: Uuid,
    ) -> Result<(), DomainError> {
        self.links
            .lock()
            .unwrap()
            .retain(|pair| *pair != (release_id, incident_id));
        Ok(())
    }

    async fn list_linked_incident_ids(&self, release_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .filter(|(r, _)| *r == release_id)
            .map(|(_, i)| *i)
            .collect())
    }

    async fn count_active_linked_incidents(&self, release_id: Uuid) -> Result<u64, DomainError> {
        let links = self.links.lock().unwrap();
        let mut active = 0u64;
        for (_, incident_id) in links.iter().filter(|(r, _)| *r == release_id) {
            if let Some(status) = self.incidents.status_of(*incident_id) {
                if status != IncidentStatus::Resolved {
                    active += 1;
                }
            }
        }
        Ok(active)
    }

    async fn list_release_states_linked_to_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState)>, DomainError> {
        let releases = self.releases.lock().unwrap();
        Ok(self
            .links
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, i)| *i == incident_id)
            .filter_map(|(r, _)| releases.get(r).map(|rel| (*r, rel.team_id, rel.base_state)))
            .collect())
    }
}
