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
        let code = self.code();
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
            DomainError::InvalidReaction => {
                (StatusCode::BAD_REQUEST, "Reaction emoji is invalid")
            }
            DomainError::TeamNotFound => (
                StatusCode::NOT_FOUND,
                "No team matches this invitation code",
            ),
            DomainError::IncidentNotFound => (StatusCode::NOT_FOUND, "Incident was not found"),
            DomainError::MemberNotFound => {
                (StatusCode::NOT_FOUND, "User is not a member of this team")
            }
            DomainError::UserNotFound => (StatusCode::NOT_FOUND, "User account not found"),
            DomainError::AlreadyMember => (
                StatusCode::CONFLICT,
                "User is already a member of this team",
            ),
            DomainError::AlreadyManager => {
                (StatusCode::CONFLICT, "User is already the team manager")
            }
            DomainError::InvalidRole => {
                (StatusCode::BAD_REQUEST, "Role must be Observer or Responder")
            }
            DomainError::CannotChangeManagerRole => (
                StatusCode::CONFLICT,
                "The manager's role can only change through a transfer",
            ),
            DomainError::NotManager => (
                StatusCode::FORBIDDEN,
                "Only the team manager may perform this action",
            ),
            DomainError::CannotModerateSelf => {
                (StatusCode::FORBIDDEN, "You cannot moderate yourself")
            }
            DomainError::CannotModerateManager => (
                StatusCode::FORBIDDEN,
                "The team manager cannot be kicked or banned",
            ),
            DomainError::UserBanned => (StatusCode::FORBIDDEN, "You are banned from this team"),
            DomainError::InvalidBanExpiry => (
                StatusCode::BAD_REQUEST,
                "A temporary ban must expire in the future",
            ),
            DomainError::InvalidBanKind => (
                StatusCode::BAD_REQUEST,
                "Ban kind must be temporary or permanent",
            ),
            DomainError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You are not allowed to perform this action",
            ),
            DomainError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "Invalid webhook signature")
            }
            DomainError::UnknownService => (StatusCode::NOT_FOUND, "Unknown webhook service"),
            DomainError::InvalidServiceSecret => {
                (StatusCode::BAD_REQUEST, "Service secret cannot be empty")
            }
            DomainError::Crypto => (StatusCode::INTERNAL_SERVER_ERROR, "Cryptographic failure"),
            DomainError::ReactionFailed => (StatusCode::BAD_GATEWAY, "Automation reaction failed"),
            DomainError::OAuthNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "OAuth provider is not configured",
            ),
            DomainError::OAuthFailed => (StatusCode::BAD_GATEWAY, "OAuth authentication failed"),
            DomainError::GiphyNotConfigured => {
                (StatusCode::SERVICE_UNAVAILABLE, "GIF search is not configured")
            }
            DomainError::ExternalServiceUnavailable => {
                (StatusCode::BAD_GATEWAY, "An external service is unavailable")
            }
            DomainError::InvalidGifQuery => (StatusCode::BAD_REQUEST, "GIF search query is invalid"),
            DomainError::AssigneeNotResponder => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Assignee must be a Responder or Manager of the team",
            ),
            DomainError::ManagerCannotLeave => (
                StatusCode::CONFLICT,
                "The team manager cannot leave the team, transfer the role or delete the team instead",
            ),
            DomainError::MustTransferManagerFirst => (
                StatusCode::CONFLICT,
                "Transfer the Manager role (or delete the station) before deleting your account",
            ),
            DomainError::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure"),
        };

        let body = Json(json!({
            "error": error_message,
            "code": code,
        }));

        (status, body).into_response()
    }
}
