use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opswarden_server::adapters::automation::StaticRuleRepo;
use opswarden_server::adapters::crypto::hmac::HmacSha256Verifier;
use opswarden_server::adapters::webhook::github::GithubParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::incident::{Incident, IncidentStatus};
use opswarden_server::domain::private_message::PrivateMessage;
use opswarden_server::domain::release::{Release, ReleaseState};
use opswarden_server::domain::team::{Role, Team, TeamBan, TeamMemberView};
use opswarden_server::domain::timeline::{ReactionRecord, TimelineEntry};
use opswarden_server::domain::user::User;
use opswarden_server::ports::{
    Clock, GifResult, GifSearch, IncidentRepo, Notifier, OAuthClient, OAuthProfile, PasswordHasher,
    PrivateMessageRepo, ReleaseRepo, RuleRepo, SecretVault, TeamRepo, TimelineRepo, TokenClaims,
    TokenRevocationRepo, TokenService, UserRepo,
};
use opswarden_server::{build_app, config::Config, AppState};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestContext {
    pub app: axum::Router,
    pub users: Arc<DummyUserRepo>,
    pub teams: Arc<DummyTeamRepo>,
    pub incidents: Arc<DummyIncidentRepo>,
    pub timeline: Arc<DummyTimelineRepo>,
    pub private_messages: Arc<DummyPrivateMessageRepo>,
    pub releases: Arc<DummyReleaseRepo>,
    pub revoked_tokens: Arc<DummyTokenRevocationRepo>,
    pub vault: Arc<DummyVault>,
}

/// In-memory secret vault for tests: stores plaintext keyed by service, so a
/// webhook test can seed a known secret and then sign a body with it.
#[derive(Default)]
pub struct DummyVault {
    secrets: Mutex<HashMap<String, String>>,
}

#[allow(dead_code)]
impl DummyVault {
    pub fn seed(&self, service: &str, secret: &str) {
        self.secrets
            .lock()
            .unwrap()
            .insert(service.to_string(), secret.to_string());
    }
}

#[async_trait]
impl SecretVault for DummyVault {
    async fn store(&self, service: &str, secret: &str) -> Result<(), DomainError> {
        self.seed(service, secret);
        Ok(())
    }

    async fn reveal(&self, service: &str) -> Result<Option<String>, DomainError> {
        Ok(self.secrets.lock().unwrap().get(service).cloned())
    }

    async fn delete(&self, service: &str) -> Result<(), DomainError> {
        self.secrets.lock().unwrap().remove(service);
        Ok(())
    }
}

/// No-op notifier for tests: the Notify REAction is unit-tested in the use-case
/// with a recording mock; here we only need the wiring to compile and succeed.
pub struct DummyNotifier;

#[async_trait]
impl Notifier for DummyNotifier {
    async fn notify(&self, _url: &str, _message: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct DummyGifSearch;

#[async_trait]
impl GifSearch for DummyGifSearch {
    async fn search(
        &self,
        query: &str,
        _limit: u32,
        _rating: &str,
    ) -> Result<Vec<GifResult>, DomainError> {
        Ok(vec![GifResult {
            id: "demo".to_string(),
            title: format!("result for {query}"),
            url: "https://media.giphy.com/media/demo/giphy.gif".to_string(),
            preview_url: "https://media.giphy.com/media/demo/200w_s.gif".to_string(),
            width: 200,
            height: 150,
        }])
    }
}

#[derive(Default)]
pub struct DummyUserRepo {
    /// Extra users seeded by tests (e.g. a private-message recipient). The
    /// default authenticated user is the nil UUID, handled below without seeding.
    extra: Mutex<HashMap<Uuid, User>>,
}

#[allow(dead_code)]
impl DummyUserRepo {
    pub fn seed_user(&self, user: User) {
        self.extra.lock().unwrap().insert(user.id, user);
    }
}

#[async_trait]
impl UserRepo for DummyUserRepo {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError> {
        if let Some(user) = self.extra.lock().unwrap().get(&user_id) {
            return Ok(Some(user.clone()));
        }
        if user_id == Uuid::nil() {
            let email = opswarden_server::domain::user::Email::new("existing@test.com").unwrap();
            Ok(Some(User {
                id: user_id,
                email,
                password_hash: "hash".to_string(),
                created_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        if email == "existing@test.com" {
            let e = opswarden_server::domain::user::Email::new(email.to_string()).unwrap();
            Ok(Some(User::new(e, "hash")))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, _user: &User) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError> {
        if user_id == Uuid::nil() {
            Ok(())
        } else {
            Err(DomainError::InvalidToken)
        }
    }
}

pub struct DummyHasher;

impl PasswordHasher for DummyHasher {
    fn hash(&self, _password: &str) -> Result<String, DomainError> {
        Ok("dummy_hash".to_string())
    }

    fn verify(&self, password: &str, _hash: &str) -> Result<bool, DomainError> {
        Ok(password == "correct_password")
    }
}

pub struct DummyTokenService;

impl TokenService for DummyTokenService {
    fn generate_token(&self, _user_id: uuid::Uuid) -> Result<String, DomainError> {
        Ok("mock_jwt_token".to_string())
    }

    fn verify_token(&self, token: &str) -> Result<TokenClaims, DomainError> {
        if token == "mock_jwt_token" {
            Ok(TokenClaims {
                user_id: uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
                expires_at: Utc::now() + chrono::Duration::hours(24),
            })
        } else {
            Err(DomainError::InvalidToken)
        }
    }
}

pub struct DummyOAuthClient;

#[async_trait]
impl OAuthClient for DummyOAuthClient {
    fn is_configured(&self) -> bool {
        true
    }

    fn authorization_url(&self, state: &str) -> Result<String, DomainError> {
        Ok(format!("https://accounts.google.test/auth?state={state}"))
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthProfile, DomainError> {
        Ok(OAuthProfile {
            email: "google@test.com".to_string(),
        })
    }
}

#[derive(Default)]
pub struct DummyTokenRevocationRepo {
    revoked: Mutex<HashSet<String>>,
}

#[async_trait]
impl TokenRevocationRepo for DummyTokenRevocationRepo {
    async fn revoke(&self, token: &str, _expires_at: DateTime<Utc>) -> Result<(), DomainError> {
        self.revoked.lock().unwrap().insert(token.to_string());
        Ok(())
    }

    async fn is_revoked(&self, token: &str) -> Result<bool, DomainError> {
        Ok(self.revoked.lock().unwrap().contains(token))
    }
}

#[derive(Default)]
pub struct DummyTeamRepo {
    teams_by_code: Mutex<HashMap<String, Team>>,
    roles: Mutex<HashMap<(Uuid, Uuid), Role>>,
    bans: Mutex<HashMap<(Uuid, Uuid), TeamBan>>,
}

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

#[async_trait]
impl TeamRepo for DummyTeamRepo {
    async fn save_team(&self, team: &Team) -> Result<(), DomainError> {
        self.seed_team(team.clone());
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

    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError> {
        Ok(self.bans.lock().unwrap().get(&(team_id, user_id)).cloned())
    }

    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBan>, DomainError> {
        Ok(self
            .bans
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.team_id == team_id)
            .cloned()
            .collect())
    }
}

pub struct DummyClock;

impl Clock for DummyClock {}

#[derive(Default)]
pub struct DummyIncidentRepo {
    incidents: Mutex<HashMap<Uuid, Incident>>,
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
}

#[async_trait]
impl IncidentRepo for DummyIncidentRepo {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        self.seed_incident(incident.clone());
        Ok(())
    }

    async fn find_incident_by_id(
        &self,
        incident_id: Uuid,
    ) -> Result<Option<Incident>, DomainError> {
        Ok(self.incidents.lock().unwrap().get(&incident_id).cloned())
    }

    async fn update_incident(&self, incident: &Incident) -> Result<(), DomainError> {
        self.seed_incident(incident.clone());
        Ok(())
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
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError> {
        let mut entries: Vec<_> = self
            .entries
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
}

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

pub fn test_context() -> TestContext {
    build_context(Arc::new(StaticRuleRepo::empty()))
}

/// A context whose hook engine has the Phase-2 GitHub rule wired to `team_id`.
/// Seed the matching secret with `ctx.vault.seed("github", secret)`.
#[allow(dead_code)]
pub fn test_context_with_github_rule(team_id: Uuid) -> TestContext {
    build_context(Arc::new(StaticRuleRepo::github_ci_to_incident(team_id)))
}

fn build_context(rules: Arc<dyn RuleRepo + Send + Sync>) -> TestContext {
    let users = Arc::new(DummyUserRepo::default());
    let teams = Arc::new(DummyTeamRepo::default());
    let incidents = Arc::new(DummyIncidentRepo::default());
    let timeline = Arc::new(DummyTimelineRepo::default());
    let private_messages = Arc::new(DummyPrivateMessageRepo::default());
    let releases = Arc::new(DummyReleaseRepo::new(incidents.clone()));
    let revoked_tokens = Arc::new(DummyTokenRevocationRepo::default());
    let vault = Arc::new(DummyVault::default());
    let config = Config::from_env();

    let app = build_app(AppState {
        users: users.clone(),
        teams: teams.clone(),
        incidents: incidents.clone(),
        timeline: timeline.clone(),
        private_messages: private_messages.clone(),
        releases: releases.clone(),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        oauth: Arc::new(DummyOAuthClient),
        token_revocations: revoked_tokens.clone(),
        events: Arc::new(WsHub::new()),
        clock: Arc::new(DummyClock),
        vault: vault.clone(),
        webhook_verifier: Arc::new(HmacSha256Verifier),
        webhook_parser: Arc::new(GithubParser),
        rules,
        notifier: Arc::new(DummyNotifier),
        gifs: Arc::new(DummyGifSearch),
        config,
    });

    TestContext {
        app,
        users,
        teams,
        incidents,
        timeline,
        private_messages,
        releases,
        revoked_tokens,
        vault,
    }
}

#[allow(dead_code)]
pub fn test_app() -> axum::Router {
    test_context().app
}
