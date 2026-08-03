// --- server/src/handlers/middleware.rs ---
use crate::adapters::rate_limit::Decision;
use crate::AppState;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header::RETRY_AFTER, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use std::net::SocketAddr;
use uuid::Uuid;

/// Bounds credential guessing on the unauthenticated `/api/auth/*` routes.
///
/// The caller is identified with the same `resolve_client_ip` the rest of the
/// server uses, so the configured proxy depth decides how far an
/// `X-Forwarded-For` chain is trusted. Without that, a single reverse proxy
/// would share one bucket across every user, and a spoofed header would let a
/// caller mint a fresh budget per request.
pub async fn rate_limit_auth(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let caller =
        super::resolve_client_ip(peer.ip(), req.headers(), state.config.trusted_proxy_hops);

    match state.auth_rate_limiter.check(caller, Utc::now()) {
        Decision::Allow => next.run(req).await,
        Decision::Deny {
            retry_after_seconds,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            [(RETRY_AFTER, retry_after_seconds.to_string())],
        )
            .into_response(),
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub user_id: Uuid,
    pub bearer_token: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.trim_start_matches("Bearer ").to_string()
        }
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let claims = state
        .tokens
        .verify_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if state
        .token_revocations
        .is_revoked(&token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    req.extensions_mut().insert(AuthenticatedSession {
        user_id: claims.user_id,
        bearer_token: token,
        expires_at: claims.expires_at,
    });

    Ok(next.run(req).await)
}
