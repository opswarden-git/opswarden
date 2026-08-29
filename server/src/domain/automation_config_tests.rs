use super::*;
use serde_json::json;

#[test]
fn all_credential_kinds_are_listed_and_round_trip_their_stored_name() {
    let mut tags: Vec<usize> = CredentialKind::ALL
        .iter()
        .map(|kind| kind.exhaustiveness_guard())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(
        tags.len(),
        10,
        "CredentialKind::ALL must list every variant exactly once"
    );

    for kind in CredentialKind::ALL {
        assert_eq!(
            CredentialKind::from_stored(&kind.to_string()).unwrap(),
            *kind
        );
    }
}

#[test]
fn connection_normalizes_provider_name() {
    let connection = ServiceConnection::new(Uuid::new_v4(), " GitHub ", Uuid::new_v4()).unwrap();
    assert_eq!(connection.service, "github");
}

#[test]
fn new_rule_is_disabled_and_requires_object_configs() {
    let team_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let connection_id = Uuid::new_v4();
    let mut rule = AutomationRule::new(
        team_id,
        "CI failed",
        connection_id,
        "ci_failed",
        json!({"repository": "opswarden/app"}),
        "create_incident",
        None,
        json!({"severity": "high"}),
        user_id,
    )
    .unwrap();
    assert!(!rule.enabled);
    let initial_revision = rule.updated_at;
    rule.set_enabled(false);
    assert!(rule.updated_at > initial_revision);

    assert_eq!(
        AutomationRule::new(
            team_id,
            "bad",
            connection_id,
            "ci_failed",
            json!([]),
            "create_incident",
            None,
            json!({}),
            user_id,
        )
        .unwrap_err(),
        DomainError::InvalidAutomationRule
    );

    assert_eq!(
        AutomationRule::new(
            team_id,
            "leaky",
            connection_id,
            "ci_failed",
            json!({"nested": {"access_token": "must-not-live-here"}}),
            "create_incident",
            None,
            json!({}),
            user_id,
        )
        .unwrap_err(),
        DomainError::InvalidAutomationRule
    );
}

#[test]
fn delivery_and_run_are_single_transition_state_machines() {
    let mut delivery = WebhookDelivery::new(Uuid::new_v4(), "delivery-1", "workflow_run").unwrap();
    delivery.mark_processed().unwrap();
    assert_eq!(
        delivery.mark_failed("late_failure").unwrap_err(),
        DomainError::InvalidAutomationTransition
    );

    let mut run = AutomationRun::new(delivery.id, Uuid::new_v4());
    let incident_id = Uuid::new_v4();
    run.mark_succeeded(Some(incident_id)).unwrap();
    assert_eq!(run.incident_id, Some(incident_id));
    assert!(run.finished_at.is_some());
    assert_eq!(
        run.mark_skipped().unwrap_err(),
        DomainError::InvalidAutomationTransition
    );
}
