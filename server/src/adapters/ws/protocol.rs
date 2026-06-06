// --- server/src/adapters/ws/protocol.rs ---

use serde_json::json;
use uuid::Uuid;

use crate::domain::event::DomainEvent;

/// Serialize a `presence_update` frame: who is currently watching `incident_id`.
/// Presence is ephemeral transport state (it lives in the hub, never the domain),
/// so its wire shape is defined here alongside the domain-event serialization.
pub fn presence_wire(incident_id: Uuid, watchers: &[Uuid]) -> String {
    json!({
        "type": "presence_update",
        "incident_id": incident_id,
        "watchers": watchers,
    })
    .to_string()
}

/// Serialize a domain event to its on-the-wire JSON, per the WebSocket contract
/// documented in `WEBSOCKET_SPEC.md`. The wire format is a transport concern and
/// lives here, never in the domain.
pub fn to_wire(event: &DomainEvent) -> String {
    let value = match event {
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
    };
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::incident::{IncidentStatus, Severity};
    use chrono::TimeZone;
    use serde_json::Value;
    use uuid::Uuid;

    fn parse(event: &DomainEvent) -> Value {
        serde_json::from_str(&to_wire(event)).unwrap()
    }

    #[test]
    fn state_changed_wire_shape() {
        let incident_id = Uuid::new_v4();
        let by = Uuid::new_v4();
        let v = parse(&DomainEvent::IncidentStateChanged {
            team_id: Uuid::new_v4(),
            incident_id,
            new_status: IncidentStatus::Acknowledged,
            by,
        });
        assert_eq!(v["type"], "incident_state_changed");
        assert_eq!(v["incident_id"], incident_id.to_string());
        assert_eq!(v["new_state"], "acknowledged");
        assert_eq!(v["by"], by.to_string());
    }

    #[test]
    fn escalated_wire_shape() {
        let v = parse(&DomainEvent::IncidentEscalated {
            team_id: Uuid::new_v4(),
            incident_id: Uuid::new_v4(),
            new_severity: Severity::Critical,
            by: Uuid::new_v4(),
        });
        assert_eq!(v["type"], "incident_escalated");
        assert_eq!(v["new_severity"], "critical");
    }

    #[test]
    fn assigned_wire_shape() {
        let assigned_to = Uuid::new_v4();
        let v = parse(&DomainEvent::IncidentAssigned {
            team_id: Uuid::new_v4(),
            incident_id: Uuid::new_v4(),
            assigned_to,
            by: Uuid::new_v4(),
        });
        assert_eq!(v["type"], "incident_assigned");
        assert_eq!(v["assigned_to"], assigned_to.to_string());
    }

    #[test]
    fn timeline_entry_added_nests_entry_with_unix_time() {
        let at = Utc.with_ymd_and_hms(2026, 6, 6, 3, 51, 44).unwrap();
        let v = parse(&DomainEvent::TimelineEntryAdded {
            team_id: Uuid::new_v4(),
            incident_id: Uuid::new_v4(),
            entry_id: Uuid::new_v4(),
            content: "Investigating".to_string(),
            author: Uuid::new_v4(),
            at,
        });
        assert_eq!(v["type"], "timeline_entry_added");
        assert_eq!(v["entry"]["content"], "Investigating");
        assert_eq!(v["entry"]["at"], at.timestamp());
    }

    use chrono::Utc;

    #[test]
    fn presence_update_wire_shape() {
        let incident_id = Uuid::new_v4();
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        let v: Value = serde_json::from_str(&presence_wire(incident_id, &[u1, u2])).unwrap();
        assert_eq!(v["type"], "presence_update");
        assert_eq!(v["incident_id"], incident_id.to_string());
        let watchers = v["watchers"].as_array().unwrap();
        assert_eq!(watchers.len(), 2);
        assert_eq!(watchers[0], u1.to_string());
    }
}
