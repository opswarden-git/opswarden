// --- server/src/domain/user.rs ---

use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::error::DomainError;

#[derive(Debug, Clone, PartialEq)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let s = value.into();
        if s.contains('@') {
            Ok(Self(s))
        } else {
            Err(DomainError::InvalidEmail)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub email: Email,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: Email, password_hash: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash: password_hash.into(),
            created_at: Utc::now(),
        }
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_valid_is_accepted() {
        let email = Email::new("test@opswarden.com");
        
        assert!(email.is_ok());
        assert_eq!(email.unwrap().as_str(), "test@opswarden.com");
    }

    #[test]
    fn email_without_at_symbol_is_rejected() {
        let result = Email::new("invalid-email.com");
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::InvalidEmail);
    }

    #[test]
    fn user_creation_generates_id_and_date() {
        let email = Email::new("admin@opswarden.com").unwrap();
        let user = User::new(email.clone(), "hashed_password");

        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, "hashed_password");
        assert_eq!(user.id.to_string().len(), 36);
        let now = Utc::now();
        assert!(now.signed_duration_since(user.created_at).num_seconds() < 2);
    }
}
