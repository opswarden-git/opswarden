// --- server/src/domain/mod.rs ---

pub mod error;
pub mod team;
pub mod user;

pub use error::DomainError;
pub use team::{
    plan_manager_transfer, InvitationCode, ManagerTransfer, Role, RoleChange, Team, TeamMember,
};
pub use user::{Email, User};
