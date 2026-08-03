// --- server/src/adapters/ws/protocol.rs ---

use serde_json::json;
use uuid::Uuid;

use crate::domain::event::DomainEvent;

/// Serialize a `presence_update` frame: who is currently watching a resource.
/// Presence is ephemeral transport state (it lives in the hub, never the domain),
/// so its wire shape is defined here alongside the domain-event serialization.
pub fn presence_wire(resource_id: Uuid, resource_type: &str, watchers: &[Uuid]) -> String {
    json!({
        "type": "presence_update",
        "resource_id": resource_id,
        "resource_type": resource_type,
        "watchers": watchers,
    })
    .to_string()
}

/// Serialize a `team_presence_update` frame: which members of `team_id` are
/// currently connected (distinct users — multiple tabs count once). Ephemeral
/// hub state, scoped to the team: only members of `team_id` ever receive it.
pub fn team_presence_wire(team_id: Uuid, online_user_ids: &[Uuid]) -> String {
    json!({
        "type": "team_presence_update",
        "team_id": team_id,
        "online_user_ids": online_user_ids,
    })
    .to_string()
}

/// Serialize a domain event to its on-the-wire JSON, per the WebSocket contract
/// documented in `WEBSOCKET_SPEC.md`. The wire format is a transport concern and
/// lives here, never in the domain.
pub fn to_wire(event: &DomainEvent) -> String {
    let value = match event {
        DomainEvent::IncidentCreated {
            incident_id,
            severity,
            ..
        } => json!({
            "type": "incident_created",
            "incident_id": incident_id,
            "severity": severity.to_string(),
        }),
        DomainEvent::IncidentStateChanged {
            incident_id,
            new_status,
            by,
            ..
        } => json!({
            "type": "incident_state_changed",
            "incident_id": incident_id,
            "new_state": new_status.to_string(),
            "by": by,
        }),
        DomainEvent::IncidentEscalated {
            incident_id,
            new_severity,
            by,
            ..
        } => json!({
            "type": "incident_escalated",
            "incident_id": incident_id,
            "new_severity": new_severity.to_string(),
            "by": by,
        }),
        DomainEvent::IncidentAssigned {
            incident_id,
            assigned_to,
            by,
            ..
        } => json!({
            "type": "incident_assigned",
            "incident_id": incident_id,
            "assigned_to": assigned_to,
            "by": by,
        }),
        DomainEvent::TimelineEntryAdded {
            incident_id,
            entry_id,
            content,
            author,
            at,
            ..
        } => json!({
            "type": "timeline_entry_added",
            "incident_id": incident_id,
            "entry": {
                "entry_id": entry_id,
                "content": content,
                "author": author,
                "at": at.timestamp(),
            },
        }),
        DomainEvent::TimelineEntryEdited {
            incident_id,
            entry_id,
            content,
            edited_at,
            ..
        } => json!({
            "type": "timeline_entry_edited",
            "incident_id": incident_id,
            "entry_id": entry_id,
            "new_content": content,
            "edited_at": edited_at.timestamp(),
        }),
        DomainEvent::ReactionAdded {
            incident_id,
            entry_id,
            emoji,
            user_id,
            ..
        } => json!({
            "type": "reaction_added",
            "incident_id": incident_id,
            "entry_id": entry_id,
            "emoji": emoji,
            "by": user_id,
        }),
        DomainEvent::ReactionRemoved {
            incident_id,
            entry_id,
            emoji,
            user_id,
            ..
        } => json!({
            "type": "reaction_removed",
            "incident_id": incident_id,
            "entry_id": entry_id,
            "emoji": emoji,
            "by": user_id,
        }),
        DomainEvent::UserTyping {
            incident_id,
            user_id,
            ..
        } => json!({
            "type": "user_typing",
            "incident_id": incident_id,
            "user_id": user_id,
        }),
        DomainEvent::RuleTriggered {
            service,
            rule_name,
            result,
            incident_id,
            ..
        } => json!({
            "type": "rule_triggered",
            "service": service,
            "rule_name": rule_name,
            "result": result.to_string(),
            "incident_id": incident_id,
        }),
        DomainEvent::RuleFailed {
            service,
            rule_name,
            error,
            ..
        } => json!({
            "type": "rule_failed",
            "service": service,
            "rule_name": rule_name,
            "error": error,
        }),
        DomainEvent::MemberKicked {
            team_id,
            member,
            by,
        } => json!({
            "type": "member_kicked",
            "team_id": team_id,
            "member": member,
            "by": by,
        }),
        DomainEvent::MemberBanned {
            team_id,
            member,
            until,
            by,
        } => json!({
            "type": "member_banned",
            "team_id": team_id,
            "member": member,
            "until": until.map(|value| value.timestamp()),
            "by": by,
        }),
        DomainEvent::PrivateMessageReceived {
            message_id: _,
            sender_id,
            recipient_id,
            content,
            at,
        } => json!({
            "type": "private_message_received",
            "from": sender_id,
            "to": recipient_id,
            "content": content,
            "at": at.timestamp(),
        }),
        DomainEvent::ReleaseStepValidated {
            release_id,
            step,
            by,
            ..
        } => json!({
            "type": "release_step_validated",
            "release_id": release_id,
            "step": step,
            "by": by,
        }),
        DomainEvent::ReleaseStateChanged {
            release_id,
            new_state,
            ..
        } => json!({
            "type": "release_state_changed",
            "release_id": release_id,
            "new_state": new_state.to_string(),
        }),
    };
    value.to_string()
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
