// server/src/handlers/auth.rs

use crate::app::auth::{
    DeleteAccountCommand, DeleteAccountUseCase, LogoutCommand, LogoutUseCase, OAuthSignInCommand,
    OAuthSignInUseCase, SignInCommand, SignInUseCase, SignUpCommand, SignUpUseCase,
};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;
use axum::{
    extract::{Extension, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SignUpPayload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SignUpResponse {
    pub email: String,
    pub status: String,
}

pub async fn sign_up(
    State(state): State<AppState>,
    Json(payload): Json<SignUpPayload>,
) -> Result<(StatusCode, Json<SignUpResponse>), DomainError> {
    let use_case = SignUpUseCase::new(state.users.clone(), state.hasher.clone());

    let command = SignUpCommand {
        email: payload.email,
        plain_password: payload.password,
    };

    // La magie opère ici : si use_case.sign_up retourne une erreur (DomainError),
    // le "?" fait remonter l'erreur, et Axum la transformera automatiquement en HTTP
    // grâce à notre implémentation de IntoResponse dans error.rs !
    let result = use_case.sign_up(command).await?;

    Ok((
        StatusCode::CREATED,
        Json(SignUpResponse {
            email: result.email,
            status: "created".to_string(),
        }),
    ))
}

#[derive(Deserialize)]
pub struct SignInPayload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SignInResponse {
    pub token: String,
}

pub async fn sign_in(
    State(state): State<AppState>,
    Json(payload): Json<SignInPayload>,
) -> Result<(StatusCode, Json<SignInResponse>), DomainError> {
    let use_case = SignInUseCase::new(
        state.users.clone(),
        state.hasher.clone(),
        state.tokens.clone(),
    );

    let command = SignInCommand {
        email: payload.email,
        plain_password: payload.password,
    };

    let result = use_case.sign_in(command).await?;

    Ok((
        StatusCode::OK,
        Json(SignInResponse {
            token: result.token,
        }),
    ))
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: uuid::Uuid,
    pub email: String,
}

pub async fn get_me(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
) -> Result<Json<MeResponse>, DomainError> {
    let user = state
        .users
        .find_by_id(session.user_id)
        .await?
        .ok_or(DomainError::InvalidToken)?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email.as_str().to_string(),
    }))
}

pub async fn delete_me(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
) -> Result<StatusCode, DomainError> {
    let use_case = DeleteAccountUseCase::new(state.users.clone(), state.teams.clone());
    use_case
        .delete_account(DeleteAccountCommand {
            user_id: session.user_id,
        })
        .await?;
    let logout = LogoutUseCase::new(state.token_revocations.clone());
    logout
        .logout(LogoutCommand {
            token: session.bearer_token,
            expires_at: session.expires_at,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
) -> Result<StatusCode, DomainError> {
    let use_case = LogoutUseCase::new(state.token_revocations.clone());

    use_case
        .logout(LogoutCommand {
            token: session.bearer_token,
            expires_at: session.expires_at,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct GoogleStartQuery {
    pub locale: Option<String>,
}

pub async fn google_start(
    State(state): State<AppState>,
    Query(query): Query<GoogleStartQuery>,
) -> Result<Response, DomainError> {
    if !state.oauth.is_configured() {
        return Err(DomainError::OAuthNotConfigured);
    }

    let locale = match query.locale.as_deref() {
        Some("fr") => "fr",
        _ => "en",
    };
    let state_token = format!("{}:{locale}", Uuid::new_v4());
    let auth_url = state.oauth.authorization_url(&state_token)?;

    let mut response = Redirect::temporary(&auth_url).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "opswarden_oauth_state={state_token}; HttpOnly; SameSite=Lax; Path=/api/auth/google; Max-Age=600"
        ))
        .map_err(|_| DomainError::OAuthFailed)?,
    );
    Ok(response)
}

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub async fn google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<Response, DomainError> {
    if query.error.is_some() {
        return Err(DomainError::OAuthFailed);
    }

    let code = query.code.ok_or(DomainError::OAuthFailed)?;
    let returned_state = query.state.ok_or(DomainError::OAuthFailed)?;
    let cookie_state =
        read_cookie(&headers, "opswarden_oauth_state").ok_or(DomainError::OAuthFailed)?;

    if cookie_state != returned_state {
        return Err(DomainError::OAuthFailed);
    }

    let profile = state.oauth.exchange_code(&code).await?;
    let locale = cookie_state
        .split_once(':')
        .map(|(_, locale)| locale)
        .filter(|locale| matches!(*locale, "en" | "fr"))
        .unwrap_or("en");
    let use_case = OAuthSignInUseCase::new(
        state.users.clone(),
        state.hasher.clone(),
        state.tokens.clone(),
    );
    let result = use_case
        .sign_in(OAuthSignInCommand {
            email: profile.email,
        })
        .await?;

    let target = format!(
        "{}/{locale}/login#oauth_token={}",
        state.config.web_origin.trim_end_matches('/'),
        result.token
    );
    let mut response = Redirect::temporary(&target).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "opswarden_oauth_state=; HttpOnly; SameSite=Lax; Path=/api/auth/google; Max-Age=0",
        ),
    );
    Ok(response)
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}
