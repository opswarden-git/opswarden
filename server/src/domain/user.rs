// --- server/src/domain/user.rs ---

use super::error::DomainError;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Fr,
}

impl Locale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }
}

impl TryFrom<&str> for Locale {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "en" => Ok(Self::En),
            "fr" => Ok(Self::Fr),
            _ => Err(DomainError::InvalidLocale),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let s = value.into().to_lowercase();
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
    pub locale: Locale,
    pub created_at: DateTime<Utc>,
}

/// Safe identity projection for API read models. Password and account metadata
/// never leak through incident, timeline, or team resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub user_id: Uuid,
    pub email: String,
}

impl From<&User> for UserSummary {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.id,
            email: user.email.as_str().to_string(),
        }
    }
}

impl User {
    pub fn new(email: Email, password_hash: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash: password_hash.into(),
            locale: Locale::En,
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
        assert_eq!(user.locale, Locale::En);
        assert_eq!(user.id.to_string().len(), 36);
        let now = Utc::now();
        assert!(now.signed_duration_since(user.created_at).num_seconds() < 2);
    }

    #[test]
    fn locale_is_strictly_limited_to_english_and_french() {
        assert_eq!(Locale::try_from("en"), Ok(Locale::En));
        assert_eq!(Locale::try_from("fr"), Ok(Locale::Fr));
        assert_eq!(Locale::try_from("de"), Err(DomainError::InvalidLocale));
        assert_eq!(Locale::try_from("FR"), Err(DomainError::InvalidLocale));
    }
}
