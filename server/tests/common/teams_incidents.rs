#[derive(Default)]
pub struct DummyTeamRepo {
    teams_by_code: Mutex<HashMap<String, Team>>,
    roles: Mutex<HashMap<(Uuid, Uuid), Role>>,
    bans: Mutex<HashMap<(Uuid, Uuid), TeamBan>>,
    images: Mutex<HashMap<Uuid, opswarden_server::domain::team::TeamImage>>,
}

#[async_trait]
impl TeamRepo for DummyTeamRepo {
    async fn create_team_with_manager(
        &self,
        team: &Team,
        manager_id: Uuid,
    ) -> Result<(), DomainError> {
        self.seed_team(team.clone());
        self.seed_member(team.id, manager_id, Role::Manager);
        Ok(())
    }

    async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError> {
        Ok(self.teams_by_code.lock().unwrap().get(code).cloned())
    }

    async fn find_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, DomainError> {
        Ok(self.role_for(team_id, user_id))
    }

    async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError> {
        self.seed_member(team_id, user_id, role);
        Ok(())
    }

    async fn transfer_manager(
        &self,
        team_id: Uuid,
        old_manager: Uuid,
        new_manager: Uuid,
    ) -> Result<(), DomainError> {
        let mut roles = self.roles.lock().unwrap();
        roles.insert((team_id, old_manager), Role::Responder);
        roles.insert((team_id, new_manager), Role::Manager);
        Ok(())
    }

    async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .keys()
            .filter(|(_, u)| *u == user_id)
            .map(|(t, _)| *t)
            .collect())
    }

    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>, DomainError> {
        let roles = self.roles.lock().unwrap();
        let teams = self.teams_by_code.lock().unwrap();
        Ok(roles
            .iter()
            .filter(|((_, u), _)| *u == user_id)
            .filter_map(|((team_id, _), role)| {
                teams
                    .values()
                    .find(|team| team.id == *team_id)
                    .map(|team| (team.clone(), *role))
            })
            .collect())
    }

    async fn list_team_directory_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TeamDirectoryItem>, DomainError> {
        let member_count_by_team = self.roles.lock().unwrap().clone();
        Ok(self
            .list_teams_for_user(user_id)
            .await?
            .into_iter()
            .map(|(team, role)| TeamDirectoryItem {
                member_count: member_count_by_team
                    .keys()
                    .filter(|(team_id, _)| *team_id == team.id)
                    .count() as u64,
                team,
                role,
                active_incident_count: 0,
                active_release_count: 0,
                blocked_release_count: 0,
                image_updated_at: None,
            })
            .collect())
    }

    async fn find_team_by_id(&self, team_id: Uuid) -> Result<Option<Team>, DomainError> {
        Ok(self
            .teams_by_code
            .lock()
            .unwrap()
            .values()
            .find(|team| team.id == team_id)
            .cloned())
    }

    async fn delete_team(&self, team_id: Uuid) -> Result<(), DomainError> {
        self.teams_by_code
            .lock()
            .unwrap()
            .retain(|_, team| team.id != team_id);
        self.roles.lock().unwrap().retain(|(t, _), _| *t != team_id);
        Ok(())
    }

    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.roles.lock().unwrap().remove(&(team_id, user_id));
        Ok(())
    }

    async fn kick_member_and_clear_assignments(
        &self,
        team_id: Uuid,
        _requester_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), DomainError> {
        self.roles.lock().unwrap().remove(&(team_id, target_user_id));
        Ok(())
    }

    async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .keys()
            .filter(|(t, _)| *t == team_id)
            .count() as u64)
    }

    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
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
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError> {
        self.roles.lock().unwrap().insert((team_id, user_id), role);
        Ok(())
    }

    async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError> {
        self.bans
            .lock()
            .unwrap()
            .insert((ban.team_id, ban.user_id), ban.clone());
        Ok(())
    }

    async fn ban_member_and_clear_assignments(
        &self,
        ban: &TeamBan,
        _requester_id: Uuid,
    ) -> Result<bool, DomainError> {
        self.bans
            .lock()
            .unwrap()
            .insert((ban.team_id, ban.user_id), ban.clone());
        Ok(self
            .roles
            .lock()
            .unwrap()
            .remove(&(ban.team_id, ban.user_id))
            .is_some())
    }

    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError> {
        Ok(self.bans.lock().unwrap().get(&(team_id, user_id)).cloned())
    }

    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
        Ok(self
            .bans
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.team_id == team_id)
            .cloned()
            .map(|ban| TeamBanView {
                user_email: format!("user-{}@test.local", ban.user_id),
                moderator_email: ban.created_by.map(|id| format!("user-{id}@test.local")),
                ban,
            })
            .collect())
    }

    async fn remove_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        self.bans.lock().unwrap().remove(&(team_id, user_id));
        Ok(())
    }

    async fn save_team_image(
        &self,
        team_id: Uuid,
        image: &opswarden_server::domain::team::TeamImage,
    ) -> Result<(), DomainError> {
        self.images.lock().unwrap().insert(team_id, image.clone());
        Ok(())
    }

    async fn find_team_image_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<opswarden_server::domain::team::TeamImage>, DomainError> {
        if self.role_for(team_id, user_id).is_none() {
            return Ok(None);
        }
        Ok(self.images.lock().unwrap().get(&team_id).cloned())
    }

    async fn delete_team_image(&self, team_id: Uuid) -> Result<(), DomainError> {
        self.images.lock().unwrap().remove(&team_id);
        Ok(())
    }
}

#[derive(Default)]
pub struct DummyIncidentRepo {
    incidents: Mutex<HashMap<Uuid, Incident>>,
    events: Mutex<Vec<IncidentEvent>>,
    reads: Mutex<HashMap<(Uuid, Uuid), DateTime<Utc>>>,
}

impl DummyIncidentRepo {
    pub fn seed_incident(&self, incident: Incident) {
        self.incidents.lock().unwrap().insert(incident.id, incident);
    }

    /// Current status of a stored incident, for the release blocking computation.
    pub fn status_of(&self, incident_id: Uuid) -> Option<IncidentStatus> {
        self.incidents
            .lock()
            .unwrap()
            .get(&incident_id)
            .map(|incident| incident.status)
    }

    pub fn record_events(&self, events: &[IncidentEvent]) {
        self.events.lock().unwrap().extend(events.iter().cloned());
    }
}

#[async_trait]
impl IncidentRepo for DummyIncidentRepo {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        self.seed_incident(incident.clone());
        Ok(())
    }

    async fn save_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        self.seed_incident(incident.clone());
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn find_incident_by_id(
        &self,
        incident_id: Uuid,
    ) -> Result<Option<Incident>, DomainError> {
        Ok(self.incidents.lock().unwrap().get(&incident_id).cloned())
    }

    async fn update_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
        _expected_updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError> {
        self.seed_incident(incident.clone());
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }

    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<opswarden_server::ports::ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError> {
        let mut events: Vec<_> = self
            .events
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
            .lock()
            .unwrap()
            .values()
            .filter(|incident| incident.team_id == team_id)
            .cloned()
            .collect())
    }

    async fn mark_incident_read(
        &self,
        incident_id: Uuid,
        user_id: Uuid,
        read_through: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.reads
            .lock()
            .unwrap()
            .entry((incident_id, user_id))
            .and_modify(|current| *current = (*current).max(read_through))
            .or_insert(read_through);
        Ok(())
    }

    async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError> {
        self.incidents.lock().unwrap().remove(&incident_id);
        Ok(())
    }

    async fn clear_assignee_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut incidents = self.incidents.lock().unwrap();
        for incident in incidents.values_mut() {
            if incident.team_id == team_id && incident.assignee == Some(user_id) {
                incident.assignee = None;
            }
        }
        Ok(())
    }
}


