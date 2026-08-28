impl DummyTeamRepo {
    pub fn seed_team(&self, team: Team) {
        self.teams_by_code
            .lock()
            .unwrap()
            .insert(team.invitation_code.as_str().to_string(), team);
    }

    pub fn seed_member(&self, team_id: Uuid, user_id: Uuid, role: Role) {
        self.roles.lock().unwrap().insert((team_id, user_id), role);
    }

    // Only the team moderation tests use this; other integration crates share
    // `common` but never seed a ban.
    #[allow(dead_code)]
    pub fn seed_ban(&self, ban: TeamBan) {
        self.bans
            .lock()
            .unwrap()
            .insert((ban.team_id, ban.user_id), ban);
    }

    pub fn role_for(&self, team_id: Uuid, user_id: Uuid) -> Option<Role> {
        self.roles.lock().unwrap().get(&(team_id, user_id)).copied()
    }
}

pub struct DummyClock;

impl Clock for DummyClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}
