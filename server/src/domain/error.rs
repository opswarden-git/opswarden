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
    Forbidden,
    /// Webhook HMAC signature missing or invalid (Phase 2).
    InvalidSignature,
    /// No secret/config registered for the targeted webhook service (Phase 2).
    UnknownService,
    /// Encryption/decryption failure in the secret vault (Phase 2).
    Crypto,
    /// A rule's outbound reaction (e.g. an HTTP/Slack notification) failed (Phase 2).
    ReactionFailed,
    Storage,
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
            DomainError::Forbidden => write!(f, "You are not allowed to perform this action"),
            DomainError::InvalidSignature => write!(f, "Invalid webhook signature"),
            DomainError::UnknownService => write!(f, "Unknown webhook service"),
            DomainError::Crypto => write!(f, "Cryptographic failure"),
            DomainError::ReactionFailed => write!(f, "Automation reaction failed"),
            DomainError::Storage => write!(f, "Storage failure"),
        }
    }
}
impl std::error::Error for DomainError {}
