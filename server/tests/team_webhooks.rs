mod common;

use std::collections::HashSet;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::test_context;
use opswarden_server::adapters::crypto::hmac::hmac_sha256;
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection, WebhookDeliveryStatus,
};
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET_A: &str = "team-a-signing-secret";
const SECRET_B: &str = "team-b-signing-secret";
const FAILED_RUN: &str = r#"{
    "repository":{"full_name":"opswarden/app"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"failure",
        "html_url":"https://github.com/opswarden/app/actions/runs/42"
    }
}"#;

fn signature(secret: &str, body: &str) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256(secret.as_bytes(), body.as_bytes()))
    )
}

fn webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    secret: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/github/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-GitHub-Delivery", delivery_id)
        .header("X-GitHub-Event", event)
        .header("X-Hub-Signature-256", signature(secret, body))
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_automation(
    ctx: &common::TestContext,
    team_id: Uuid,
    secret: &str,
    trigger_config: Value,
    reaction_kind: &str,
) -> (ServiceConnection, AutomationRule) {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(connection.id, CredentialKind::WebhookSigningSecret, secret)
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "GitHub CI failed",
        connection.id,
        "ci_failed",
        trigger_config,
        reaction_kind,
        None,
        json!({"severity": "critical"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (connection, rule)
}

async fn seed_http_automation(
    ctx: &common::TestContext,
    team_id: Uuid,
    secret: &str,
) -> (ServiceConnection, ServiceConnection, AutomationRule) {
    let user_id = Uuid::new_v4();
    let github = ServiceConnection::new(team_id, "github", user_id).unwrap();
    let http = ServiceConnection::new(team_id, "http", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&github)
        .await
        .unwrap();
    ctx.service_connections
        .insert_connection(&http)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(github.id, CredentialKind::WebhookSigningSecret, secret)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            http.id,
            CredentialKind::EndpointUrl,
            "https://hooks.example.com/opswarden-secret",
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "GitHub CI failed to HTTP",
        github.id,
        "ci_failed",
        json!({}),
        "http_notify",
        Some(http.id),
        json!({}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (github, http, rule)
}

#[tokio::test]
async fn signed_delivery_creates_incident_and_durable_run_then_duplicate_is_noop() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (connection, _) = seed_automation(
        &ctx,
        team_id,
        SECRET_A,
        json!({"repository": "opswarden/app", "branch": "main"}),
        "vigil_create_incident",
    )
    .await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    ctx.events
        .register(Uuid::new_v4(), HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["duplicate"], false);
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].title, "CI failed on opswarden/app");
    assert!(incidents[0].description.contains("Branch: main"));
    assert_eq!(incidents[0].severity.to_string(), "critical");

    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, WebhookDeliveryStatus::Processed);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(runs[0].incident_id, Some(incidents[0].id));
    assert!(rx.try_recv().unwrap().contains("rule_triggered"));

    let persisted_connection = ctx
        .service_connections
        .find_connection_by_id(connection.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted_connection.verified_at.is_some());
    assert!(persisted_connection.last_delivery_at.is_some());
    assert_eq!(persisted_connection.last_error_code, None);

    let duplicate = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let duplicate = json_body(duplicate).await;
    assert_eq!(duplicate["duplicate"], true);
    assert_eq!(duplicate["rules_triggered"], 0);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(ctx.automation_runs.all().len(), 1);
}

#[tokio::test]
async fn signed_delivery_notifies_http_once_and_persists_a_successful_run() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, http, _) = seed_http_automation(&ctx, team_id, SECRET_A).await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "http-delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(
        ctx.notifier.calls()[0].0,
        "https://hooks.example.com/opswarden-secret"
    );
    assert!(ctx.notifier.calls()[0]
        .1
        .contains("CI failed on opswarden/app"));
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(runs[0].incident_id, None);
    let persisted_http = ctx
        .service_connections
        .find_connection_by_id(http.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted_http.verified_at.is_some());
    assert!(persisted_http.last_delivery_at.is_none());

    let duplicate = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "http-delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(duplicate).await["duplicate"], true);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(ctx.automation_runs.all().len(), 1);
}

#[tokio::test]
async fn failed_http_reaction_does_not_block_the_vigil_reaction() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, http, _) = seed_http_automation(&ctx, team_id, SECRET_A).await;
    let mut vigil_rule = AutomationRule::new(
        team_id,
        "GitHub CI failed to VIGIL",
        github.id,
        "ci_failed",
        json!({}),
        "vigil_create_incident",
        None,
        json!({"severity": "high"}),
        Uuid::new_v4(),
    )
    .unwrap();
    vigil_rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&vigil_rule).await.unwrap();
    ctx.notifier.fail_requests();

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "mixed-delivery",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 1);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap()
            .len(),
        1
    );
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 2);
    assert!(runs
        .iter()
        .any(|run| run.status == AutomationRunStatus::Succeeded));
    assert!(runs.iter().any(|run| {
        run.status == AutomationRunStatus::Failed
            && run.error_code.as_deref() == Some("reaction_http_5xx")
    }));
    let persisted_http = ctx
        .service_connections
        .find_connection_by_id(http.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted_http.last_error_code.as_deref(),
        Some("reaction_http_5xx")
    );
}

#[tokio::test]
async fn connection_secret_and_rules_are_isolated_between_teams() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    let (connection_a, _) =
        seed_automation(&ctx, team_a, SECRET_A, json!({}), "vigil_create_incident").await;
    seed_automation(&ctx, team_b, SECRET_B, json!({}), "vigil_create_incident").await;

    let wrong_secret = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection_a.id,
            "wrong-secret",
            "workflow_run",
            SECRET_B,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(wrong_secret.status(), StatusCode::UNAUTHORIZED);
    assert!(ctx.webhook_deliveries.all().is_empty());

    let accepted = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection_a.id,
            "team-a-delivery",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_a)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_b)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn signed_ping_verifies_connection_without_running_rules() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (connection, _) =
        seed_automation(&ctx, team_id, SECRET_A, json!({}), "vigil_create_incident").await;
    let ping = r#"{"zen":"Keep it logically awesome."}"#;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "ping-1",
            "ping",
            SECRET_A,
            ping,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(
        ctx.webhook_deliveries.all()[0].status,
        WebhookDeliveryStatus::Ignored
    );
    assert!(ctx.automation_runs.all().is_empty());
    assert!(ctx
        .service_connections
        .find_connection_by_id(connection.id)
        .await
        .unwrap()
        .unwrap()
        .verified_at
        .is_some());
}

#[tokio::test]
async fn filter_mismatch_creates_no_run_and_unsupported_reaction_records_failure() {
    let ctx = test_context();
    let filtered_team = Uuid::new_v4();
    let (filtered_connection, _) = seed_automation(
        &ctx,
        filtered_team,
        SECRET_A,
        json!({"repository": "another/project"}),
        "vigil_create_incident",
    )
    .await;
    let ignored = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            filtered_connection.id,
            "filtered",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(ignored).await["rules_triggered"], 0);
    assert!(ctx.automation_runs.all().is_empty());

    let failing_team = Uuid::new_v4();
    let (failing_connection, _) =
        seed_automation(&ctx, failing_team, SECRET_B, json!({}), "http_notify").await;
    let failed = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            failing_connection.id,
            "unsupported",
            "workflow_run",
            SECRET_B,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(failed).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["rules_failed"], 1);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Failed);
    assert_eq!(
        runs[0].error_code.as_deref(),
        Some("invalid_automation_rule")
    );
    assert!(ctx
        .incidents
        .list_incidents_for_team(failing_team)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn provider_headers_are_required_and_body_is_limited() {
    let ctx = test_context();
    let missing_header = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/webhooks/github/{}", Uuid::new_v4()))
                .header("X-GitHub-Event", "workflow_run")
                .body(Body::from(FAILED_RUN))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_header.status(), StatusCode::BAD_REQUEST);

    let oversized = "x".repeat(1024 * 1024 + 1);
    let too_large = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/webhooks/github/{}", Uuid::new_v4()))
                .header("X-GitHub-Delivery", "large")
                .header("X-GitHub-Event", "workflow_run")
                .header("X-Hub-Signature-256", "sha256=deadbeef")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
