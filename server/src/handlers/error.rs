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
            DomainError::InvalidPrivateMessage => {
                (StatusCode::BAD_REQUEST, "Private message content is invalid")
            }
            DomainError::NoSharedTeam => (
                StatusCode::FORBIDDEN,
                "You can only message members of a team you share",
            ),
            DomainError::InvalidReleaseTitle => {
                (StatusCode::BAD_REQUEST, "Release title cannot be empty")
            }
            DomainError::InvalidReleaseSteps => (
                StatusCode::BAD_REQUEST,
                "A release needs at least one distinct, non-empty step",
            ),
            DomainError::ReleaseNotFound => (StatusCode::NOT_FOUND, "Release was not found"),
            DomainError::InvalidReleaseStep => {
                (StatusCode::CONFLICT, "Steps must be validated in order")
            }
            DomainError::ReleaseBlocked => (
                StatusCode::CONFLICT,
                "The release is blocked by an active linked incident",
            ),
            DomainError::InvalidReleaseTransition => {
                (StatusCode::CONFLICT, "Invalid release lifecycle transition")
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
            DomainError::InvalidServiceConnection => {
                (StatusCode::BAD_REQUEST, "Service connection is invalid")
            }
            DomainError::ServiceConnectionNotFound => {
                (StatusCode::NOT_FOUND, "Service connection was not found")
            }
            DomainError::InvalidAutomationRule => {
                (StatusCode::BAD_REQUEST, "Automation rule is invalid")
            }
            DomainError::AutomationRuleNotFound => {
                (StatusCode::NOT_FOUND, "Automation rule was not found")
            }
            DomainError::InvalidAutomationRun => {
                (StatusCode::BAD_REQUEST, "Automation run is invalid")
            }
            DomainError::InvalidWebhookDelivery => {
                (StatusCode::BAD_REQUEST, "Webhook delivery is invalid")
            }
            DomainError::InvalidAutomationTransition => (
                StatusCode::CONFLICT,
                "Automation resource is already terminal",
            ),
            DomainError::Crypto => (StatusCode::INTERNAL_SERVER_ERROR, "Cryptographic failure"),
            DomainError::ReactionFailed => (StatusCode::BAD_GATEWAY, "Automation reaction failed"),
            DomainError::InvalidReactionEndpoint => {
                (StatusCode::BAD_REQUEST, "Reaction endpoint is invalid")
            }
            DomainError::UnsafeReactionTarget => {
                (StatusCode::BAD_REQUEST, "Reaction target is not allowed")
            }
            DomainError::ReactionTimeout => (StatusCode::GATEWAY_TIMEOUT, "Reaction timed out"),
            DomainError::ReactionResponseTooLarge => (
                StatusCode::BAD_GATEWAY,
                "Reaction response exceeded the size limit",
            ),
            DomainError::ReactionPayloadTooLarge => (
                StatusCode::BAD_REQUEST,
                "Reaction payload exceeded the size limit",
            ),
            DomainError::ReactionRedirectRefused => {
                (StatusCode::BAD_GATEWAY, "Reaction redirect was refused")
            }
            DomainError::ReactionHttp4xx => {
                (StatusCode::BAD_GATEWAY, "Reaction endpoint rejected the request")
            }
            DomainError::ReactionHttp5xx => {
                (StatusCode::BAD_GATEWAY, "Reaction endpoint failed")
            }
            DomainError::ReactionNetworkError => {
                (StatusCode::BAD_GATEWAY, "Reaction network request failed")
            }
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
