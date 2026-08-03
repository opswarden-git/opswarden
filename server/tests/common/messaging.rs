#[allow(dead_code)]
#[derive(Default)]
pub struct DummyPrivateMessageRepo {
    messages: Mutex<Vec<PrivateMessage>>,
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
        user_a: Uuid,
        user_b: Uuid,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError> {
        let mut msgs: Vec<PrivateMessage> = self
            .messages
            .lock()
            .unwrap()
            .iter()
            .filter(|m| {
                (m.sender_id == user_a && m.recipient_id == user_b)
                    || (m.sender_id == user_b && m.recipient_id == user_a)
            })
            .cloned()
            .collect();
        msgs.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        msgs.truncate(limit as usize);
        Ok(msgs)
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

    async fn update_release(&self, release: &Release) -> Result<(), DomainError> {
        self.releases
            .lock()
            .unwrap()
            .insert(release.id, release.clone());
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
    ) -> Result<Vec<(Uuid, Uuid, ReleaseState)>, DomainError> {
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
