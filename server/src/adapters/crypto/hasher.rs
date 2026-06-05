// server/src/adapters/crypto/hasher.rs
use crate::domain::error::DomainError;
use crate::ports::PasswordHasher;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as Argon2PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub struct Argon2Hasher;

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| DomainError::InvalidCredentials)? // Ideally InternalError
            .to_string();

        Ok(password_hash)
    }

    fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|_| DomainError::InvalidCredentials)?;
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }
}
