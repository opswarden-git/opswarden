// server/src/handlers/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use crate::domain::error::DomainError;

impl IntoResponse for DomainError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            DomainError::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            DomainError::InvalidEmail => (StatusCode::BAD_REQUEST, "Invalid email address"),
            DomainError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid email or password"),
            DomainError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
