// --- server/src/domain/error.rs ---

#[derive(Debug, PartialEq)]
pub enum DomainError {
    InvalidEmail,
    UserAlreadyExists,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::InvalidEmail => write!(f, "Invalid email address"),
            DomainError::UserAlreadyExists => write!(f, "User already exists"),
        }
    }
}
impl std::error::Error for DomainError {}