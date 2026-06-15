// --- server/src/domain/mod.rs ---

pub mod automation;
pub mod error;
pub mod event;
pub mod incident;
pub mod team;
pub mod timeline;
pub mod user;

pub use automation::{evaluate, ExternalEvent, Reaction, Rule};
pub use error::DomainError;
pub use event::DomainEvent;
pub use incident::{Incident, IncidentStatus, Severity};
pub use team::{
    plan_manager_transfer, InvitationCode, ManagerTransfer, Role, RoleChange, Team, TeamMember,
};
pub use timeline::{TimelineEntry, MAX_TIMELINE_ENTRY_LEN};
pub use user::{Email, User};
