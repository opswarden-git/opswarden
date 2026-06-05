// --- server/src/domain/error.rs ---

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidEmail,
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::InvalidEmail => write!(f, "Invalid email format"),
            DomainError::UserAlreadyExists => write!(f, "User already exists"),
            DomainError::InvalidCredentials => write!(f, "Invalid email or password"),
            DomainError::InvalidToken => write!(f, "Invalid or expired token"),
        }
    }
}
impl std::error::Error for DomainError {}