use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::team::{Role, Team};
use opswarden_server::domain::user::User;
use opswarden_server::ports::{
    Clock, PasswordHasher, TeamRepo, TokenClaims, TokenRevocationRepo, TokenService, UserRepo,
};
use opswarden_server::{build_app, config::Config, AppState};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestContext {
    pub app: axum::Router,
    pub teams: Arc<DummyTeamRepo>,
    pub revoked_tokens: Arc<DummyTokenRevocationRepo>,
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
}

pub struct DummyClock;

impl Clock for DummyClock {}

pub fn test_context() -> TestContext {
    let teams = Arc::new(DummyTeamRepo::default());
    let revoked_tokens = Arc::new(DummyTokenRevocationRepo::default());
    let config = Config::from_env();

    let app = build_app(AppState {
        users: Arc::new(DummyUserRepo),
        teams: teams.clone(),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        token_revocations: revoked_tokens.clone(),
        clock: Arc::new(DummyClock),
        config,
    });

    TestContext {
        app,
        teams,
        revoked_tokens,
    }
}

#[allow(dead_code)]
pub fn test_app() -> axum::Router {
    test_context().app
}
