// --- server/src/domain/error.rs ---

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidEmail,
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
    InvalidTeamName,
    InvalidIncidentTitle,
    InvalidIncidentStatus,
    InvalidSeverity,
    InvalidIncidentTransition,
    InvalidTimelineEntry,
    TeamNotFound,
    IncidentNotFound,
    AlreadyMember,
    MemberNotFound,
    NotManager,
    AlreadyManager,
    AssigneeNotResponder,
    ManagerCannotLeave,
    /// Account deletion refused while the user still manages a team: they must
    /// transfer the Manager role (or delete the station) first.
    MustTransferManagerFirst,
    Forbidden,
    /// Webhook HMAC signature missing or invalid (Phase 2).
    InvalidSignature,
    /// No secret/config registered for the targeted webhook service (Phase 2).
    UnknownService,
    /// Encryption/decryption failure in the secret vault (Phase 2).
    Crypto,
    /// A rule's outbound reaction (e.g. an HTTP/Slack notification) failed (Phase 2).
    ReactionFailed,
    OAuthNotConfigured,
    OAuthFailed,
    Storage,
}

impl DomainError {
    /// Stable, machine-readable error code (snake_case). Decouples clients from
    /// the human-readable message: the wording can change without breaking the
    /// frontend's `errors.<code>` i18n mapping.
    pub fn code(&self) -> &'static str {
        match self {
            DomainError::InvalidEmail => "invalid_email",
            DomainError::UserAlreadyExists => "user_already_exists",
            DomainError::InvalidCredentials => "invalid_credentials",
            DomainError::InvalidToken => "invalid_token",
            DomainError::InvalidTeamName => "invalid_team_name",
            DomainError::InvalidIncidentTitle => "invalid_incident_title",
            DomainError::InvalidIncidentStatus => "invalid_incident_status",
            DomainError::InvalidSeverity => "invalid_severity",
            DomainError::InvalidIncidentTransition => "invalid_incident_transition",
            DomainError::InvalidTimelineEntry => "invalid_timeline_entry",
            DomainError::TeamNotFound => "team_not_found",
            DomainError::IncidentNotFound => "incident_not_found",
            DomainError::AlreadyMember => "already_member",
            DomainError::MemberNotFound => "member_not_found",
            DomainError::NotManager => "not_manager",
            DomainError::AlreadyManager => "already_manager",
            DomainError::AssigneeNotResponder => "assignee_not_responder",
            DomainError::ManagerCannotLeave => "manager_cannot_leave",
            DomainError::MustTransferManagerFirst => "must_transfer_manager_first",
            DomainError::Forbidden => "forbidden",
            DomainError::InvalidSignature => "invalid_signature",
            DomainError::UnknownService => "unknown_service",
            DomainError::Crypto => "crypto_error",
            DomainError::ReactionFailed => "reaction_failed",
            DomainError::OAuthNotConfigured => "oauth_not_configured",
            DomainError::OAuthFailed => "oauth_failed",
            DomainError::Storage => "storage_error",
        }
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::InvalidEmail => write!(f, "Invalid email format"),
            DomainError::UserAlreadyExists => write!(f, "User already exists"),
            DomainError::InvalidCredentials => write!(f, "Invalid email or password"),
            DomainError::InvalidToken => write!(f, "Invalid or expired token"),
            DomainError::InvalidTeamName => write!(f, "Team name cannot be empty"),
            DomainError::InvalidIncidentTitle => write!(f, "Incident title cannot be empty"),
            DomainError::InvalidIncidentStatus => write!(f, "Incident status is invalid"),
            DomainError::InvalidSeverity => write!(f, "Incident severity is invalid"),
            DomainError::InvalidIncidentTransition => {
                write!(f, "Invalid incident lifecycle transition")
            }
            DomainError::InvalidTimelineEntry => {
                write!(f, "Timeline entry content is invalid")
            }
            DomainError::TeamNotFound => write!(f, "No team matches this invitation code"),
            DomainError::IncidentNotFound => write!(f, "Incident was not found"),
            DomainError::AlreadyMember => write!(f, "User is already a member of this team"),
            DomainError::MemberNotFound => write!(f, "User is not a member of this team"),
            DomainError::NotManager => write!(f, "Only the team manager may perform this action"),
            DomainError::AlreadyManager => write!(f, "User is already the team manager"),
            DomainError::AssigneeNotResponder => {
                write!(f, "Assignee must be a Responder or Manager of the team")
            }
            DomainError::ManagerCannotLeave => write!(f, "The team manager cannot leave the team, transfer the role or delete the team instead"),
            DomainError::MustTransferManagerFirst => write!(f, "Transfer the Manager role (or delete the station) before deleting your account"),
            DomainError::Forbidden => write!(f, "You are not allowed to perform this action"),
            DomainError::InvalidSignature => write!(f, "Invalid webhook signature"),
            DomainError::UnknownService => write!(f, "Unknown webhook service"),
            DomainError::Crypto => write!(f, "Cryptographic failure"),
            DomainError::ReactionFailed => write!(f, "Automation reaction failed"),
            DomainError::OAuthNotConfigured => write!(f, "OAuth provider is not configured"),
            DomainError::OAuthFailed => write!(f, "OAuth authentication failed"),
            DomainError::Storage => write!(f, "Storage failure"),
        }
    }
}
impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::DomainError;

    #[test]
    fn codes_are_stable_and_snake_case() {
        assert_eq!(
            DomainError::InvalidCredentials.code(),
            "invalid_credentials"
        );
        assert_eq!(
            DomainError::MustTransferManagerFirst.code(),
            "must_transfer_manager_first"
        );
        for err in [
            DomainError::Forbidden,
            DomainError::Storage,
            DomainError::OAuthFailed,
        ] {
            let code = err.code();
            assert!(!code.is_empty());
            assert!(code.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_'));
        }
    }
}
