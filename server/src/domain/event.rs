// --- server/src/domain/event.rs ---

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::incident::{IncidentStatus, Severity};

/// Business events worth broadcasting in real time. These are domain-level facts
/// ("an incident was acknowledged"), not a wire format: the WebSocket adapter
/// serializes them to the on-the-wire JSON. Every event carries `team_id` so the
/// broadcaster can fan out to the connected members of that team without knowing
/// any business rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    IncidentStateChanged {
        team_id: Uuid,
        incident_id: Uuid,
        new_status: IncidentStatus,
        by: Uuid,
    },
    IncidentEscalated {
        team_id: Uuid,
        incident_id: Uuid,
        new_severity: Severity,
        by: Uuid,
    },
    IncidentAssigned {
        team_id: Uuid,
        incident_id: Uuid,
        assigned_to: Uuid,
        by: Uuid,
    },
    TimelineEntryAdded {
        team_id: Uuid,
        incident_id: Uuid,
        entry_id: Uuid,
        content: String,
        author: Uuid,
        at: DateTime<Utc>,
    },
    UserTyping {
        team_id: Uuid,
        incident_id: Uuid,
        user_id: Uuid,
    },
}

impl DomainEvent {
    /// The team whose connected members should receive this event.
    pub fn team_id(&self) -> Uuid {
        match self {
            DomainEvent::IncidentStateChanged { team_id, .. }
            | DomainEvent::IncidentEscalated { team_id, .. }
            | DomainEvent::IncidentAssigned { team_id, .. }
            | DomainEvent::TimelineEntryAdded { team_id, .. }
            | DomainEvent::UserTyping { team_id, .. } => *team_id,
        }
    }
}
