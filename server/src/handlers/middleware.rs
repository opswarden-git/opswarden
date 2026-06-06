// --- server/src/handlers/middleware.rs ---
use crate::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

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
        Some(header) if header.starts_with("Bearer ") => header.trim_start_matches("Bearer "),
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    let user_id = state
        .tokens
        .verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Insert the parsed user_id into the request extensions so handlers can access it
    req.extensions_mut().insert(user_id);

    Ok(next.run(req).await)
}
