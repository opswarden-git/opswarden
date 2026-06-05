// --- server/src/domain/mod.rs ---

pub mod error;
pub mod user;

pub use error::DomainError;
pub use user::{Email, User};
