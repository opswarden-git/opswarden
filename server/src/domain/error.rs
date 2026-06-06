// --- server/src/domain/error.rs ---

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidEmail,
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
    InvalidTeamName,
    InvalidIncidentTitle,
    InvalidIncidentTransition,
    InvalidTimelineEntry,
    TeamNotFound,
    IncidentNotFound,
    AlreadyMember,
    MemberNotFound,
    NotManager,
    AlreadyManager,
    Forbidden,
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
            DomainError::Forbidden => write!(f, "You are not allowed to perform this action"),
            DomainError::Storage => write!(f, "Storage failure"),
        }
    }
}
impl std::error::Error for DomainError {}
