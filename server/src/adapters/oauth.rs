use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::ports::{OAuthClient, OAuthProfile, ServiceOAuthClient, ServiceOAuthTokens};

pub struct GoogleOAuthClient {
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: String,
}

impl GoogleOAuthClient {
    pub fn new(
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_uri: String,
    ) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: String,
    #[serde(default)]
    verified_email: bool,
}

#[async_trait]
impl OAuthClient for GoogleOAuthClient {
    fn is_configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    fn authorization_url(&self, state: &str) -> Result<String, DomainError> {
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;

        Ok(format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=online&prompt=select_account",
            percent_encode(client_id),
            percent_encode(&self.redirect_uri),
            percent_encode("openid email"),
            percent_encode(state),
        ))
    }

    async fn exchange_code(&self, code: &str) -> Result<OAuthProfile, DomainError> {
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        let client_secret = self
            .client_secret
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;

        let token = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", code),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .error_for_status()
            .map_err(|_| DomainError::OAuthFailed)?
            .json::<GoogleTokenResponse>()
            .await
            .map_err(|_| DomainError::OAuthFailed)?;

        let user = self
            .client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(token.access_token)
            .send()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .error_for_status()
            .map_err(|_| DomainError::OAuthFailed)?
            .json::<GoogleUserInfo>()
            .await
            .map_err(|_| DomainError::OAuthFailed)?;

        if !user.verified_email {
            return Err(DomainError::OAuthFailed);
        }

        Ok(OAuthProfile { email: user.email })
    }
}

/// Minimal GitHub identity flow used only to resolve a verified primary email.
/// The access token is deliberately short-lived in memory and is never stored.
pub struct GithubOAuthClient {
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: String,
}

impl GithubOAuthClient {
    pub fn new(
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_uri: String,
    ) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[derive(Deserialize)]
struct GithubIdentityEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[async_trait]
impl OAuthClient for GithubOAuthClient {
    fn is_configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    fn authorization_url(&self, state: &str) -> Result<String, DomainError> {
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        if state.trim().is_empty() {
            return Err(DomainError::OAuthFailed);
        }

        Ok(format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            percent_encode(client_id),
            percent_encode(&self.redirect_uri),
            percent_encode("user:email"),
            percent_encode(state),
        ))
    }

    async fn exchange_code(&self, code: &str) -> Result<OAuthProfile, DomainError> {
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        let client_secret = self
            .client_secret
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        if code.trim().is_empty() {
            return Err(DomainError::OAuthFailed);
        }

        let token = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("User-Agent", "OpsWarden")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .error_for_status()
            .map_err(|_| DomainError::OAuthFailed)?
            .json::<GithubTokenResponse>()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .access_token
            .filter(|token| !token.trim().is_empty())
            .ok_or(DomainError::OAuthFailed)?;

        let emails = self
            .client
            .get("https://api.github.com/user/emails")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "OpsWarden")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .error_for_status()
            .map_err(|_| DomainError::OAuthFailed)?
            .json::<Vec<GithubIdentityEmail>>()
            .await
            .map_err(|_| DomainError::OAuthFailed)?;

        let email = emails
            .into_iter()
            .find(|email| email.primary && email.verified)
            .map(|email| email.email)
            .ok_or(DomainError::OAuthFailed)?;
        Ok(OAuthProfile { email })
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// GitHub OAuth web flow used for Team-owned automation connections. Configure
/// a GitHub App with expiring user tokens to receive both access and refresh
/// tokens; ordinary OAuth Apps remain compatible but return no refresh token.
pub struct GithubServiceOAuthClient {
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: String,
}

impl GithubServiceOAuthClient {
    pub fn new(
        client_id: Option<String>,
        client_secret: Option<String>,
        redirect_uri: String,
    ) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            redirect_uri,
        }
    }

    async fn token_request(
        &self,
        parameters: &[(&str, &str)],
    ) -> Result<ServiceOAuthTokens, DomainError> {
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        let client_secret = self
            .client_secret
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        let mut form = vec![("client_id", client_id), ("client_secret", client_secret)];
        form.extend_from_slice(parameters);

        let response = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("User-Agent", "OpsWarden")
            .form(&form)
            .send()
            .await
            .map_err(|_| DomainError::OAuthFailed)?
            .error_for_status()
            .map_err(|_| DomainError::OAuthFailed)?
            .json::<GithubTokenResponse>()
            .await
            .map_err(|_| DomainError::OAuthFailed)?;

        let access_token = response.access_token.ok_or(DomainError::OAuthFailed)?;
        if access_token.trim().is_empty()
            || response
                .refresh_token
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(DomainError::OAuthFailed);
        }
        Ok(ServiceOAuthTokens {
            access_token,
            refresh_token: response.refresh_token,
        })
    }
}

#[derive(Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

#[async_trait]
impl ServiceOAuthClient for GithubServiceOAuthClient {
    fn is_configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    fn authorization_url(&self, state: &str, code_challenge: &str) -> Result<String, DomainError> {
        if !self.is_configured() {
            return Err(DomainError::OAuthNotConfigured);
        }
        let client_id = self
            .client_id
            .as_deref()
            .ok_or(DomainError::OAuthNotConfigured)?;
        if state.trim().is_empty() || code_challenge.trim().is_empty() {
            return Err(DomainError::OAuthFailed);
        }
        Ok(format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            percent_encode(client_id),
            percent_encode(&self.redirect_uri),
            percent_encode("repo"),
            percent_encode(state),
            percent_encode(code_challenge),
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<ServiceOAuthTokens, DomainError> {
        if code.trim().is_empty() || code_verifier.trim().is_empty() {
            return Err(DomainError::OAuthFailed);
        }
        self.token_request(&[
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("code_verifier", code_verifier),
        ])
        .await
    }

    async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<ServiceOAuthTokens, DomainError> {
        if refresh_token.trim().is_empty() {
            return Err(DomainError::OAuthFailed);
        }
        self.token_request(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_authorization_url_uses_state_pkce_and_minimal_repo_scope() {
        let client = GithubServiceOAuthClient::new(
            Some("github-client".to_string()),
            Some("github-secret".to_string()),
            "http://localhost:8080/api/service-oauth/github/callback".to_string(),
        );

        let url = client
            .authorization_url("unguessable-state", "pkce-challenge")
            .unwrap();

        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=github-client"));
        assert!(url.contains("scope=repo"));
        assert!(url.contains("state=unguessable-state"));
        assert!(url.contains("code_challenge=pkce-challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(!url.contains("github-secret"));
    }

    #[test]
    fn github_identity_url_requests_only_verified_email_access() {
        let client = GithubOAuthClient::new(
            Some("identity-client".to_string()),
            Some("identity-secret".to_string()),
            "http://localhost:8080/api/auth/github/callback".to_string(),
        );

        let url = client.authorization_url("identity-state").unwrap();
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=identity-client"));
        assert!(url.contains("scope=user%3Aemail"));
        assert!(url.contains("state=identity-state"));
        assert!(!url.contains("identity-secret"));
        assert!(!url.contains("scope=repo"));
    }

    #[test]
    fn github_service_oauth_requires_both_server_credentials() {
        let missing_secret = GithubServiceOAuthClient::new(
            Some("client".to_string()),
            None,
            "http://localhost/callback".to_string(),
        );
        assert!(!missing_secret.is_configured());
        assert_eq!(
            missing_secret.authorization_url("state", "challenge"),
            Err(DomainError::OAuthNotConfigured)
        );
    }
}
