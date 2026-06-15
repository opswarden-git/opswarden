// server/src/handlers/error.rs

use crate::domain::error::DomainError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

impl IntoResponse for DomainError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            DomainError::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            DomainError::InvalidEmail => (StatusCode::BAD_REQUEST, "Invalid email address"),
            DomainError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid email or password")
            }
            DomainError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            DomainError::InvalidTeamName => (StatusCode::BAD_REQUEST, "Team name cannot be empty"),
            DomainError::InvalidIncidentTitle => {
                (StatusCode::BAD_REQUEST, "Incident title cannot be empty")
            }
            DomainError::InvalidIncidentStatus => {
                (StatusCode::BAD_REQUEST, "Incident status is invalid")
            }
            DomainError::InvalidSeverity => {
                (StatusCode::BAD_REQUEST, "Incident severity is invalid")
            }
            DomainError::InvalidIncidentTransition => (
                StatusCode::BAD_REQUEST,
                "Invalid incident lifecycle transition",
            ),
            DomainError::InvalidTimelineEntry => {
                (StatusCode::BAD_REQUEST, "Timeline entry content is invalid")
            }
            DomainError::TeamNotFound => (
                StatusCode::NOT_FOUND,
                "No team matches this invitation code",
            ),
            DomainError::IncidentNotFound => (StatusCode::NOT_FOUND, "Incident was not found"),
            DomainError::MemberNotFound => {
                (StatusCode::NOT_FOUND, "User is not a member of this team")
            }
            DomainError::AlreadyMember => (
                StatusCode::CONFLICT,
                "User is already a member of this team",
            ),
            DomainError::AlreadyManager => {
                (StatusCode::CONFLICT, "User is already the team manager")
            }
            DomainError::NotManager => (
                StatusCode::FORBIDDEN,
                "Only the team manager may perform this action",
            ),
            DomainError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You are not allowed to perform this action",
            ),
            DomainError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "Invalid webhook signature")
            }
            DomainError::UnknownService => (StatusCode::NOT_FOUND, "Unknown webhook service"),
            DomainError::Crypto => (StatusCode::INTERNAL_SERVER_ERROR, "Cryptographic failure"),
            DomainError::ReactionFailed => (StatusCode::BAD_GATEWAY, "Automation reaction failed"),
            DomainError::AssigneeNotResponder => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Assignee must be a Responder or Manager of the team",
            ),
            DomainError::ManagerCannotLeave => (
                StatusCode::CONFLICT,
                "The team manager cannot leave the team, transfer the role or delete the team instead",
            ),
            DomainError::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure"),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
