// --- server/src/domain/error.rs ---

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidEmail,
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
    InvalidTeamName,
    TeamNotFound,
    AlreadyMember,
    MemberNotFound,
    NotManager,
    AlreadyManager,
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
            DomainError::TeamNotFound => write!(f, "No team matches this invitation code"),
            DomainError::AlreadyMember => write!(f, "User is already a member of this team"),
            DomainError::MemberNotFound => write!(f, "User is not a member of this team"),
            DomainError::NotManager => write!(f, "Only the team manager may perform this action"),
            DomainError::AlreadyManager => write!(f, "User is already the team manager"),
            DomainError::Storage => write!(f, "Storage failure"),
        }
    }
}
impl std::error::Error for DomainError {}
