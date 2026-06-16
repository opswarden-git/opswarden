use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::ports::{OAuthClient, OAuthProfile};

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
