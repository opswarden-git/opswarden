use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opswarden_server::adapters::automation::StaticRuleRepo;
use opswarden_server::adapters::crypto::hmac::HmacSha256Verifier;
use opswarden_server::adapters::webhook::github::GithubParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::incident::Incident;
use opswarden_server::domain::team::{Role, Team};
use opswarden_server::domain::timeline::TimelineEntry;
use opswarden_server::domain::user::User;
use opswarden_server::ports::{
    Clock, IncidentRepo, Notifier, PasswordHasher, RuleRepo, SecretVault, TeamRepo, TimelineRepo,
    TokenClaims, TokenRevocationRepo, TokenService, UserRepo,
};
use opswarden_server::{build_app, config::Config, AppState};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestContext {
    pub app: axum::Router,
    pub teams: Arc<DummyTeamRepo>,
    pub incidents: Arc<DummyIncidentRepo>,
    pub timeline: Arc<DummyTimelineRepo>,
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

pub struct DummyUserRepo;

#[async_trait]
impl UserRepo for DummyUserRepo {
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
}

#[derive(Default)]
pub struct DummyTimelineRepo {
    entries: Mutex<Vec<TimelineEntry>>,
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
    let teams = Arc::new(DummyTeamRepo::default());
    let incidents = Arc::new(DummyIncidentRepo::default());
    let timeline = Arc::new(DummyTimelineRepo::default());
    let revoked_tokens = Arc::new(DummyTokenRevocationRepo::default());
    let vault = Arc::new(DummyVault::default());
    let config = Config::from_env();

    let app = build_app(AppState {
        users: Arc::new(DummyUserRepo),
        teams: teams.clone(),
        incidents: incidents.clone(),
        timeline: timeline.clone(),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        token_revocations: revoked_tokens.clone(),
        events: Arc::new(WsHub::new()),
        clock: Arc::new(DummyClock),
        vault: vault.clone(),
        webhook_verifier: Arc::new(HmacSha256Verifier),
        webhook_parser: Arc::new(GithubParser),
        rules,
        notifier: Arc::new(DummyNotifier),
        config,
    });

    TestContext {
        app,
        teams,
        incidents,
        timeline,
        revoked_tokens,
        vault,
    }
}

#[allow(dead_code)]
pub fn test_app() -> axum::Router {
    test_context().app
}
