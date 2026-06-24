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
    /// A timeline entry's content was edited (RTC 2). `edited_at` is the new stamp.
    TimelineEntryEdited {
        team_id: Uuid,
        incident_id: Uuid,
        entry_id: Uuid,
        content: String,
        edited_at: DateTime<Utc>,
    },
    /// A user added an emoji reaction to a timeline entry (RTC 2).
    ReactionAdded {
        team_id: Uuid,
        incident_id: Uuid,
        entry_id: Uuid,
        emoji: String,
        user_id: Uuid,
    },
    /// A user removed an emoji reaction from a timeline entry (RTC 2).
    ReactionRemoved {
        team_id: Uuid,
        incident_id: Uuid,
        entry_id: Uuid,
        emoji: String,
        user_id: Uuid,
    },
    UserTyping {
        team_id: Uuid,
        incident_id: Uuid,
        user_id: Uuid,
    },
    /// An automation rule fired and its reaction succeeded (Phase 2). Carries the
    /// opened incident when the reaction created one (`CreateIncident`), `None`
    /// for side-effect reactions like `Notify`.
    RuleTriggered {
        team_id: Uuid,
        service: String,
        rule: String,
        incident_id: Option<Uuid>,
    },
    /// An automation rule matched but its reaction failed (Phase 2).
    RuleFailed {
        team_id: Uuid,
        service: String,
        rule: String,
        reason: String,
    },
    /// A member was removed from a team by moderation (kick or ban). Lets the
    /// remaining members refresh their roster, and the removed user drop the team
    /// and lose access. Covers both kick and ban — the distinction lives in the
    /// bans list, not the realtime signal.
    TeamMemberRemoved { team_id: Uuid, user_id: Uuid },
    /// A private message was sent (RTC 2). Unlike every other event this is *not*
    /// team-scoped: it must reach exactly the two participants (sender +
    /// recipient), never a whole team. See `delivery`.
    PrivateMessageReceived {
        message_id: Uuid,
        sender_id: Uuid,
        recipient_id: Uuid,
        content: String,
        at: DateTime<Utc>,
    },
}

/// How the WebSocket adapter should fan an event out. Keeps the routing rule in
/// the domain (where the event is defined) without coupling it to the transport:
/// almost everything is team-scoped, private messages are user-scoped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventDelivery {
    /// Deliver to every connected member of this team.
    Team(Uuid),
    /// Deliver only to connections owned by these users — never team-wide.
    Users(Vec<Uuid>),
}

impl DomainEvent {
    /// The set of clients that should receive this event. Team events fan out to
    /// the team's connected members; a private message reaches only its two
    /// participants.
    pub fn delivery(&self) -> EventDelivery {
        match self {
            DomainEvent::IncidentStateChanged { team_id, .. }
            | DomainEvent::IncidentEscalated { team_id, .. }
            | DomainEvent::IncidentAssigned { team_id, .. }
            | DomainEvent::TimelineEntryAdded { team_id, .. }
            | DomainEvent::TimelineEntryEdited { team_id, .. }
            | DomainEvent::ReactionAdded { team_id, .. }
            | DomainEvent::ReactionRemoved { team_id, .. }
            | DomainEvent::UserTyping { team_id, .. }
            | DomainEvent::RuleTriggered { team_id, .. }
            | DomainEvent::RuleFailed { team_id, .. }
            | DomainEvent::TeamMemberRemoved { team_id, .. } => EventDelivery::Team(*team_id),
            DomainEvent::PrivateMessageReceived {
                sender_id,
                recipient_id,
                ..
            } => EventDelivery::Users(vec![*sender_id, *recipient_id]),
        }
    }
}
