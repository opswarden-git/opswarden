// --- server/src/handlers/middleware.rs ---
use crate::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

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
