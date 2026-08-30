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

/// Coarse ceiling on the unauthenticated `/api/auth/*` routes.
///
/// The caller is identified with the same `resolve_client_ip` the rest of the
/// server uses, so the configured proxy depth decides how far an
/// `X-Forwarded-For` chain is trusted, and a spoofed header cannot mint a fresh
/// budget.
///
/// This ceiling is deliberately loose. A proxy that forwards nothing — Compose's
/// Next client is one — collapses every visitor onto its own address, and a
/// tight budget there locks out the whole deployment rather than an attacker.
/// The tight limit that actually stops credential stuffing is keyed by account
/// in the sign-in handler, where no topology can blur it.
pub async fn rate_limit_auth(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let caller =
        super::resolve_client_ip(peer.ip(), req.headers(), state.config.trusted_proxy_hops);

    match state
        .auth_rate_limiter
        .check(&format!("addr:{caller}"), state.clock.now())
    {
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
