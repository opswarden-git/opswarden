use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::app::automation::team_connection::{ALERTMANAGER_SERVICE, GITHUB_SERVICE};
use crate::app::automation::{
    CompleteGithubOAuthCommand, ConfigureEmailConnectionCommand, ConfigureGenericConnectionCommand,
    ConfigureGithubConnectionCommand, ConfigureGitlabConnectionCommand,
    ConfigureHttpConnectionCommand, DeleteTeamConnectionCommand, ListTeamConnectionsCommand,
    RefreshGithubOAuthCommand, StartGithubOAuthCommand, TeamConnectionOAuthUseCase,
    TeamConnectionUseCase, TeamConnectionView, TestConnectionCommand,
};
use crate::domain::error::DomainError;
use crate::handlers::auth::{disable_oauth_response_caching, oauth_cookie_secure_suffix};
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureGithubPayload {
    pub webhook_signing_secret: Option<String>,
    pub personal_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureGitlabPayload {
    pub webhook_signing_secret: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureGenericPayload {
    pub webhook_signing_secret: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureHttpPayload {
    pub endpoint_url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureEmailPayload {
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub from_address: Option<String>,
}

#[derive(Serialize)]
pub struct TeamConnectionResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub service: String,
    pub secret_configured: bool,
    pub token_configured: bool,
    pub oauth_configured: bool,
    pub oauth_refresh_configured: bool,
    pub endpoint_configured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub webhook_path: Option<String>,
}

impl From<TeamConnectionView> for TeamConnectionResponse {
    fn from(view: TeamConnectionView) -> Self {
        use crate::domain::automation_config::CredentialKind;

        let webhook_path = match view.connection.service.as_str() {
            "github" | "gitlab" | "generic" | "alertmanager" => Some(format!(
                "/webhooks/{}/{}",
                view.connection.service, view.connection.id
            )),
            _ => None,
        };
        Self {
            id: view.connection.id,
            team_id: view.connection.team_id,
            service: view.connection.service,
            secret_configured: view
                .configured_credentials
                .contains(&CredentialKind::WebhookSigningSecret),
            token_configured: view
                .configured_credentials
                .contains(&CredentialKind::PersonalToken),
            oauth_configured: view
                .configured_credentials
                .contains(&CredentialKind::OAuthAccessToken),
            oauth_refresh_configured: view
                .configured_credentials
                .contains(&CredentialKind::OAuthRefreshToken),
            endpoint_configured: view
                .configured_credentials
                .contains(&CredentialKind::EndpointUrl),
            created_at: view.connection.created_at,
            updated_at: view.connection.updated_at,
            verified_at: view.connection.verified_at,
            last_delivery_at: view.connection.last_delivery_at,
            last_error_code: view.connection.last_error_code,
            webhook_path,
        }
    }
}

pub async fn list_connections(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<TeamConnectionResponse>>, DomainError> {
    let views = TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
        state.email_sender.clone(),
    )
    .list(ListTeamConnectionsCommand {
        team_id,
        requester_id: session.user_id,
    })
    .await?;
    Ok(Json(views.into_iter().map(Into::into).collect()))
}

pub async fn configure_service(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, service)): Path<(Uuid, String)>,
    Json(payload): Json<Value>,
) -> Result<Json<TeamConnectionResponse>, DomainError> {
    let use_case = TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
        state.email_sender.clone(),
    );
    let view = match service.as_str() {
        GITHUB_SERVICE => {
            let payload: ConfigureGithubPayload =
                serde_json::from_value(payload).map_err(|_| DomainError::InvalidServiceSecret)?;
            use_case
                .configure_github(ConfigureGithubConnectionCommand {
                    team_id,
                    requester_id: session.user_id,
                    webhook_signing_secret: payload.webhook_signing_secret,
                    personal_token: payload.personal_token,
                })
                .await?
        }
        "gitlab" => {
            let payload: ConfigureGitlabPayload =
                serde_json::from_value(payload).map_err(|_| DomainError::InvalidServiceSecret)?;
            use_case
                .configure_gitlab(ConfigureGitlabConnectionCommand {
                    team_id,
                    requester_id: session.user_id,
                    webhook_token: payload.webhook_signing_secret,
                })
                .await?
        }
        "generic" | ALERTMANAGER_SERVICE => {
            let payload: ConfigureGenericPayload =
                serde_json::from_value(payload).map_err(|_| DomainError::InvalidServiceSecret)?;
            let command = ConfigureGenericConnectionCommand {
                team_id,
                requester_id: session.user_id,
                webhook_token: payload.webhook_signing_secret,
            };
            if service == ALERTMANAGER_SERVICE {
                use_case.configure_alertmanager(command).await?
            } else {
                use_case.configure_generic(command).await?
            }
        }
        "http" => {
            let payload: ConfigureHttpPayload = serde_json::from_value(payload)
                .map_err(|_| DomainError::InvalidReactionEndpoint)?;
            use_case
                .configure_http(ConfigureHttpConnectionCommand {
                    team_id,
                    requester_id: session.user_id,
                    endpoint_url: payload.endpoint_url,
                })
                .await?
        }
        "email" => {
            let payload: ConfigureEmailPayload =
                serde_json::from_value(payload).map_err(|_| DomainError::InvalidServiceSecret)?;
            use_case
                .configure_email(ConfigureEmailConnectionCommand {
                    team_id,
                    requester_id: session.user_id,
                    smtp_host: payload.smtp_host,
                    smtp_port: payload.smtp_port,
                    smtp_username: payload.smtp_username,
                    smtp_password: payload.smtp_password,
                    from_address: payload.from_address,
                })
                .await?
        }
        _ => return Err(DomainError::InvalidServiceConnection),
    };
    Ok(Json(view.into()))
}

#[derive(Deserialize)]
pub struct StartGithubOAuthQuery {
    pub locale: Option<String>,
}

#[derive(Serialize)]
pub struct StartGithubOAuthResponse {
    pub authorization_url: String,
}

#[derive(Deserialize, Serialize)]
struct GithubOAuthStateClaims {
    state: String,
    team_id: Uuid,
    requester_id: Uuid,
    locale: String,
    code_verifier: String,
    exp: usize,
}

pub async fn start_github_oauth(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, service)): Path<(Uuid, String)>,
    Query(query): Query<StartGithubOAuthQuery>,
) -> Result<Response, DomainError> {
    if service != GITHUB_SERVICE {
        return Err(DomainError::InvalidServiceConnection);
    }
    let locale = match query.locale.as_deref() {
        Some("fr") => "fr",
        _ => "en",
    };
    let oauth_state = Uuid::new_v4().simple().to_string();
    let code_verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
    let authorization_url = TeamConnectionOAuthUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.service_oauth.clone(),
    )
    .start(StartGithubOAuthCommand {
        team_id,
        requester_id: session.user_id,
        state: oauth_state.clone(),
        code_challenge,
    })
    .await?;

    let cookie_value = encode(
        &Header::default(),
        &GithubOAuthStateClaims {
            state: oauth_state,
            team_id,
            requester_id: session.user_id,
            locale: locale.to_string(),
            code_verifier,
            exp: (Utc::now() + Duration::minutes(10)).timestamp() as usize,
        },
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|_| DomainError::OAuthFailed)?;
    let secure = oauth_cookie_secure_suffix(&state.config.github_oauth_redirect_uri);
    let mut response = Json(StartGithubOAuthResponse { authorization_url }).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "opswarden_github_oauth={cookie_value}; HttpOnly; SameSite=Lax; \
             Path=/api/service-oauth/github; Max-Age=600{secure}"
        ))
        .map_err(|_| DomainError::OAuthFailed)?,
    );
    disable_oauth_response_caching(&mut response);
    Ok(response)
}

#[derive(Deserialize)]
pub struct GithubOAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub async fn github_oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<GithubOAuthCallbackQuery>,
) -> Result<Response, DomainError> {
    if query.error.is_some() {
        return Err(DomainError::OAuthFailed);
    }
    let code = query.code.ok_or(DomainError::OAuthFailed)?;
    let returned_state = query.state.ok_or(DomainError::OAuthFailed)?;
    let cookie = read_cookie(&headers, "opswarden_github_oauth").ok_or(DomainError::OAuthFailed)?;
    let claims = decode::<GithubOAuthStateClaims>(
        &cookie,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| DomainError::OAuthFailed)?
    .claims;
    if claims.state != returned_state
        || !matches!(claims.locale.as_str(), "en" | "fr")
        || claims.code_verifier.len() < 43
        || claims.code_verifier.len() > 128
    {
        return Err(DomainError::OAuthFailed);
    }

    TeamConnectionOAuthUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.service_oauth.clone(),
    )
    .complete(CompleteGithubOAuthCommand {
        team_id: claims.team_id,
        requester_id: claims.requester_id,
        code,
        code_verifier: claims.code_verifier,
    })
    .await?;

    let target = format!(
        "{}/{}/teams/{}/automations?view=connections&oauth=github_connected",
        state.config.web_origin.trim_end_matches('/'),
        claims.locale,
        claims.team_id
    );
    let secure = oauth_cookie_secure_suffix(&state.config.github_oauth_redirect_uri);
    let mut response = Redirect::temporary(&target).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "opswarden_github_oauth=; HttpOnly; SameSite=Lax; \
             Path=/api/service-oauth/github; Max-Age=0{secure}"
        ))
        .map_err(|_| DomainError::OAuthFailed)?,
    );
    disable_oauth_response_caching(&mut response);
    Ok(response)
}

pub async fn refresh_github_oauth(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TeamConnectionResponse>, DomainError> {
    let view = TeamConnectionOAuthUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.service_oauth.clone(),
    )
    .refresh(RefreshGithubOAuthCommand {
        team_id,
        requester_id: session.user_id,
        connection_id,
    })
    .await?;
    Ok(Json(view.into()))
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

pub async fn test_connection(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
        state.email_sender.clone(),
    )
    .test(TestConnectionCommand {
        team_id,
        requester_id: session.user_id,
        connection_id,
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_connection(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
        state.email_sender.clone(),
    )
    .delete(DeleteTeamConnectionCommand {
        team_id,
        requester_id: session.user_id,
        connection_id,
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
