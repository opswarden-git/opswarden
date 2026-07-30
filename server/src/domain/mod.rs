// --- server/src/domain/mod.rs ---

pub mod automation;
pub mod automation_catalog;
pub mod automation_config;
mod automation_credential;
pub mod automation_template;
pub mod automation_timer;
pub mod capabilities;
pub mod channel;
pub mod error;
pub mod event;
pub mod incident;
pub mod incident_event;
pub mod private_message;
pub mod release;
pub mod team;
pub mod timeline;
pub mod user;

pub use automation::ExternalEvent;
pub use automation_config::{
    AutomationRule, AutomationRuleDefinition, AutomationRun, AutomationRunStatus, CredentialKind,
    ServiceConnection, WebhookDelivery, WebhookDeliveryStatus,
};
pub use automation_timer::{
    ClaimedTimerOccurrence, TimerOccurrence, TimerSchedule, TimerScheduleProjection,
};
pub use capabilities::{derive_capabilities, TeamCapabilities};
pub use channel::Channel;
pub use error::DomainError;
pub use event::DomainEvent;
pub use incident::{Incident, IncidentStatus, Severity};
pub use incident_event::{IncidentEvent, IncidentEventKind};
pub use private_message::{PrivateMessage, MAX_PRIVATE_MESSAGE_LEN};
pub use release::{effective_release_state, Release, ReleaseState, ReleaseStep};
pub use team::{
    plan_manager_transfer, InvitationCode, ManagerTransfer, Role, RoleChange, Team, TeamMember,
};
pub use timeline::{TimelineEntry, MAX_TIMELINE_ENTRY_LEN};
pub use user::{Email, User};
