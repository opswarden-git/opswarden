#[derive(Default)]
pub struct DummyUserRepo {
    /// Extra users seeded by tests (e.g. a private-message recipient). The
    /// default authenticated user is the nil UUID, handled below without seeding.
    extra: Mutex<HashMap<Uuid, User>>,
    locales: Mutex<HashMap<Uuid, Locale>>,
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
                locale: self
                    .locales
                    .lock()
                    .unwrap()
                    .get(&user_id)
                    .copied()
                    .unwrap_or(Locale::En),
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

    async fn update_locale(&self, user_id: Uuid, locale: Locale) -> Result<(), DomainError> {
        if user_id != Uuid::nil() && !self.extra.lock().unwrap().contains_key(&user_id) {
            return Err(DomainError::UserNotFound);
        }
        self.locales.lock().unwrap().insert(user_id, locale);
        if let Some(user) = self.extra.lock().unwrap().get_mut(&user_id) {
            user.locale = locale;
        }
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

pub struct DummyGithubAuthOAuthClient;

#[async_trait]
impl OAuthClient for DummyGithubAuthOAuthClient {
    fn is_configured(&self) -> bool {
        true
    }

    fn authorization_url(&self, state: &str) -> Result<String, DomainError> {
        Ok(format!("https://github.test/login/oauth/authorize?state={state}"))
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthProfile, DomainError> {
        Ok(OAuthProfile {
            email: "github@test.com".to_string(),
        })
    }
}

#[derive(Default)]
pub struct DummyServiceOAuthClient {
    exchanges: Mutex<Vec<(String, String)>>,
    refreshes: Mutex<Vec<String>>,
}

#[allow(dead_code)]
impl DummyServiceOAuthClient {
    pub fn exchanges(&self) -> Vec<(String, String)> {
        self.exchanges.lock().unwrap().clone()
    }

    pub fn refreshes(&self) -> Vec<String> {
        self.refreshes.lock().unwrap().clone()
    }
}

#[async_trait]
impl ServiceOAuthClient for DummyServiceOAuthClient {
    fn is_configured(&self) -> bool {
        true
    }

    fn authorization_url(&self, state: &str, code_challenge: &str) -> Result<String, DomainError> {
        Ok(format!(
            "https://github.test/login/oauth/authorize?state={state}&code_challenge={code_challenge}&code_challenge_method=S256"
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<ServiceOAuthTokens, DomainError> {
        self.exchanges
            .lock()
            .unwrap()
            .push((code.to_string(), code_verifier.to_string()));
        Ok(ServiceOAuthTokens {
            access_token: "github_oauth_access_never_returned".to_string(),
            refresh_token: Some("github_oauth_refresh_never_returned".to_string()),
        })
    }

    async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<ServiceOAuthTokens, DomainError> {
        self.refreshes
            .lock()
            .unwrap()
            .push(refresh_token.to_string());
        Ok(ServiceOAuthTokens {
            access_token: "github_oauth_access_rotated".to_string(),
            refresh_token: Some("github_oauth_refresh_rotated".to_string()),
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
