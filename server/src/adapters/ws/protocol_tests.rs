use super::*;
use crate::domain::event::AutomationRuleResult;
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
fn incident_created_wire_shape_includes_direct_severity() {
    let incident_id = Uuid::new_v4();
    let value = parse(&DomainEvent::IncidentCreated {
        team_id: Uuid::new_v4(),
        incident_id,
        severity: Severity::Critical,
    });

    assert_eq!(value["type"], "incident_created");
    assert_eq!(value["incident_id"], incident_id.to_string());
    assert_eq!(value["severity"], "critical");
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
    let v: Value =
        serde_json::from_str(&presence_wire(incident_id, "incident", &[u1, u2])).unwrap();
    assert_eq!(v["type"], "presence_update");
    assert_eq!(v["resource_id"], incident_id.to_string());
    assert_eq!(v["resource_type"], "incident");
    let watchers = v["watchers"].as_array().unwrap();
    assert_eq!(watchers.len(), 2);
    assert_eq!(watchers[0], u1.to_string());
}

#[test]
fn team_presence_update_wire_shape() {
    let team_id = Uuid::new_v4();
    let u1 = Uuid::new_v4();
    let u2 = Uuid::new_v4();
    let v: Value = serde_json::from_str(&team_presence_wire(team_id, &[u1, u2])).unwrap();
    assert_eq!(v["type"], "team_presence_update");
    assert_eq!(v["team_id"], team_id.to_string());
    let online = v["online_user_ids"].as_array().unwrap();
    assert_eq!(online.len(), 2);
    assert_eq!(online[0], u1.to_string());
}

#[test]
fn cursor_update_wire_shape() {
    let incident_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let value: Value =
        serde_json::from_str(&cursor_wire(incident_id, user_id, 0.25, 0.75)).unwrap();
    assert_eq!(value["type"], "cursor_update");
    assert_eq!(value["incident_id"], incident_id.to_string());
    assert_eq!(value["user_id"], user_id.to_string());
    assert_eq!(value["x"], 0.25);
    assert_eq!(value["y"], 0.75);
}

#[test]
fn user_typing_wire_shape() {
    let incident_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let v: Value = serde_json::from_str(&user_typing_wire(incident_id, user_id)).unwrap();
    assert_eq!(v["type"], "user_typing");
    assert_eq!(v["incident_id"], incident_id.to_string());
    assert_eq!(v["user_id"], user_id.to_string());
}

#[test]
fn timeline_entry_edited_wire_shape() {
    let incident_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();
    let edited_at = Utc.with_ymd_and_hms(2026, 6, 22, 10, 0, 0).unwrap();
    let v = parse(&DomainEvent::TimelineEntryEdited {
        team_id: Uuid::new_v4(),
        incident_id,
        entry_id,
        content: "fixed typo".to_string(),
        edited_at,
    });
    assert_eq!(v["type"], "timeline_entry_edited");
    assert_eq!(v["incident_id"], incident_id.to_string());
    assert_eq!(v["entry_id"], entry_id.to_string());
    assert_eq!(v["new_content"], "fixed typo");
    assert_eq!(v["edited_at"], edited_at.timestamp());
}

#[test]
fn reaction_added_and_removed_wire_shapes() {
    let incident_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let added = parse(&DomainEvent::ReactionAdded {
        team_id: Uuid::new_v4(),
        incident_id,
        entry_id,
        emoji: "👍".to_string(),
        user_id,
    });
    assert_eq!(added["type"], "reaction_added");
    assert_eq!(added["entry_id"], entry_id.to_string());
    assert_eq!(added["emoji"], "👍");
    assert_eq!(added["by"], user_id.to_string());

    let removed = parse(&DomainEvent::ReactionRemoved {
        team_id: Uuid::new_v4(),
        incident_id,
        entry_id,
        emoji: "👍".to_string(),
        user_id,
    });
    assert_eq!(removed["type"], "reaction_removed");
    assert_eq!(removed["by"], user_id.to_string());
}

#[test]
fn rule_triggered_wire_shape() {
    let incident_id = Uuid::new_v4();
    let v = parse(&DomainEvent::RuleTriggered {
        team_id: Uuid::new_v4(),
        service: "github".to_string(),
        rule_name: "github-ci-failed-to-incident".to_string(),
        result: AutomationRuleResult::IncidentCreated,
        incident_id: Some(incident_id),
    });
    assert_eq!(
        v,
        json!({
            "type": "rule_triggered",
            "service": "github",
            "rule_name": "github-ci-failed-to-incident",
            "result": "incident_created",
            "incident_id": incident_id,
        })
    );

    let completed = parse(&DomainEvent::RuleTriggered {
        team_id: Uuid::new_v4(),
        service: "http".to_string(),
        rule_name: "notify-responders".to_string(),
        result: AutomationRuleResult::ReactionCompleted,
        incident_id: None,
    });
    assert_eq!(completed["result"], "reaction_completed");
    assert!(completed["incident_id"].is_null());
}

#[test]
fn member_kicked_wire_shape() {
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    let by = Uuid::new_v4();
    let v = parse(&DomainEvent::MemberKicked {
        team_id,
        member,
        by,
    });
    assert_eq!(v["type"], "member_kicked");
    assert_eq!(v["team_id"], team_id.to_string());
    assert_eq!(v["member"], member.to_string());
    assert_eq!(v["by"], by.to_string());
}

#[test]
fn member_banned_wire_shape_includes_nullable_until() {
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    let by = Uuid::new_v4();
    let until = Utc.with_ymd_and_hms(2026, 6, 25, 14, 30, 0).unwrap();
    let temporary = parse(&DomainEvent::MemberBanned {
        team_id,
        member,
        until: Some(until),
        by,
    });
    assert_eq!(temporary["type"], "member_banned");
    assert_eq!(temporary["team_id"], team_id.to_string());
    assert_eq!(temporary["member"], member.to_string());
    assert_eq!(temporary["until"], until.timestamp());
    assert_eq!(temporary["by"], by.to_string());

    let permanent = parse(&DomainEvent::MemberBanned {
        team_id,
        member,
        until: None,
        by,
    });
    assert!(permanent["until"].is_null());
}

#[test]
fn private_message_received_is_flat_with_unix_time() {
    let message_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let recipient_id = Uuid::new_v4();
    let at = Utc.with_ymd_and_hms(2026, 6, 24, 14, 30, 0).unwrap();
    let v = parse(&DomainEvent::PrivateMessageReceived {
        message_id,
        sender_id,
        recipient_id,
        content: "ping".to_string(),
        at,
    });
    assert_eq!(v["type"], "private_message_received");
    assert_eq!(v["from"], sender_id.to_string());
    assert_eq!(v["to"], recipient_id.to_string());
    assert_eq!(v["content"], "ping");
    assert_eq!(v["at"], at.timestamp());
    assert!(v.get("message").is_none());
    assert!(!v.to_string().contains(&message_id.to_string()));
}

#[test]
fn private_message_presence_and_typing_are_flat_and_scoped() {
    let (alice, bob) = (Uuid::new_v4(), Uuid::new_v4());
    let presence: serde_json::Value =
        serde_json::from_str(&private_message_presence_wire([alice, bob], &[alice])).unwrap();
    assert_eq!(presence["type"], "private_message_presence");
    assert_eq!(presence["participants"], serde_json::json!([alice, bob]));
    assert_eq!(presence["watchers"], serde_json::json!([alice]));

    let typing: serde_json::Value =
        serde_json::from_str(&private_message_typing_wire(alice, bob)).unwrap();
    assert_eq!(typing["type"], "private_message_typing");
    assert_eq!(typing["from"], alice.to_string());
    assert_eq!(typing["to"], bob.to_string());
}

#[test]
fn private_message_mutations_have_stable_wire_contracts() {
    let (message_id, from, to) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let at = Utc::now();
    let edited = parse(&DomainEvent::PrivateMessageEdited {
        message_id,
        sender_id: from,
        recipient_id: to,
        at,
    });
    assert_eq!(edited["type"], "private_message_edited");
    assert_eq!(edited["message_id"], message_id.to_string());
    assert_eq!(edited["at"], at.timestamp());
}

#[test]
fn release_step_validated_wire_shape() {
    let release_id = Uuid::new_v4();
    let by = Uuid::new_v4();
    let v = parse(&DomainEvent::ReleaseStepValidated {
        team_id: Uuid::new_v4(),
        release_id,
        step: "staging".to_string(),
        by,
    });
    assert_eq!(v["type"], "release_step_validated");
    assert_eq!(v["release_id"], release_id.to_string());
    assert_eq!(v["step"], "staging");
    assert_eq!(v["by"], by.to_string());
}

#[test]
fn release_state_changed_wire_shape() {
    use crate::domain::release::ReleaseState;
    let release_id = Uuid::new_v4();
    let v = parse(&DomainEvent::ReleaseStateChanged {
        team_id: Uuid::new_v4(),
        release_id,
        new_state: ReleaseState::Blocked,
    });
    assert_eq!(v["type"], "release_state_changed");
    assert_eq!(v["release_id"], release_id.to_string());
    assert_eq!(v["new_state"], "blocked");
}

#[test]
fn rule_failed_wire_shape() {
    let v = parse(&DomainEvent::RuleFailed {
        team_id: Uuid::new_v4(),
        service: "github".to_string(),
        rule_name: "github-ci-failed-to-incident".to_string(),
        error: "invalid_incident_title".to_string(),
    });
    assert_eq!(
        v,
        json!({
            "type": "rule_failed",
            "service": "github",
            "rule_name": "github-ci-failed-to-incident",
            "error": "invalid_incident_title",
        })
    );
}
