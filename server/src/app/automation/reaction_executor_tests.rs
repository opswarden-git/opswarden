use super::*;
use serde_json::{json, Map};

#[test]
fn incident_and_notification_use_normalized_provider_facts() {
    let attributes: Map<String, Value> = serde_json::from_value(json!({
        "repository": "opswarden/app",
        "workflow": "CI",
        "branch": "main",
        "conclusion": "failure",
        "run_url": "https://github.test/run/42"
    }))
    .unwrap();
    let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
    assert_eq!(default_incident_title(&event), "CI failed on opswarden/app");
    assert!(incident_description(&event).contains("Branch: main"));
    assert!(notification_text(&event).contains("Run: https://github.test/run/42"));
}

#[test]
fn notification_text_is_utf8_safe_and_bounded() {
    let mut attributes = Map::new();
    attributes.insert("repository".into(), Value::String("é".repeat(2000)));
    let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
    let text = notification_text(&event);
    assert!(text.len() <= MAX_NOTIFICATION_TEXT_BYTES);
    assert!(text.ends_with('…'));
}

#[test]
fn reaction_payload_templates_use_repository_and_workflow() {
    let attributes: Map<String, Value> = serde_json::from_value(json!({
        "repository": "opswarden/app",
        "workflow": "CI"
    }))
    .unwrap();
    let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
    assert_eq!(
        configured_title(
            &json!({"title": "[{{repository}}] {{workflow}} failed"}),
            &event
        )
        .unwrap()
        .as_deref(),
        Some("[opswarden/app] CI failed")
    );
    assert_eq!(
        configured_message_by_name(
            &json!({"message": "{{workflow}} failed on {{repository}}"}),
            "message",
            &event
        )
        .unwrap()
        .as_deref(),
        Some("CI failed on opswarden/app")
    );
}

#[test]
fn extended_github_events_have_meaningful_default_titles() {
    let cases = [
        (
            "ci_succeeded",
            json!({"repository": "opswarden/app", "workflow": "CI"}),
            "CI succeeded on opswarden/app",
        ),
        (
            "tag_pushed",
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            "Tag v1.2.3 pushed on opswarden/app",
        ),
        (
            "pr_merged",
            json!({"repository": "opswarden/app", "pull_request_number": "42"}),
            "Pull request #42 merged on opswarden/app",
        ),
    ];
    for (kind, attributes, expected) in cases {
        let attributes: Map<String, Value> = serde_json::from_value(attributes).unwrap();
        let event = ExternalEvent::new("github", kind).with_attributes(attributes);
        assert_eq!(default_incident_title(&event), expected);
    }
}
