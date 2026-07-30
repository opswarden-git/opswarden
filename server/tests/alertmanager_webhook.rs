mod common;

use axum::{body::Body, http::Request, http::StatusCode, response::Response};
use common::test_context;
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection, WebhookDeliveryStatus,
};
use opswarden_server::domain::incident::Severity;
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "alertmanager-secret";
const FIRING_GROUP: &str = r#"{
  "version":"4",
  "groupKey":"{}:{severity=\"critical\"}",
  "status":"firing",
  "receiver":"opswarden",
  "commonLabels":{"severity":"critical"},
  "alerts":[
    {"status":"firing","labels":{"alertname":"ApiDown","severity":"critical"},"annotations":{"summary":"API unavailable"}},
    {"status":"firing","labels":{"alertname":"WorkerDown","severity":"critical"},"annotations":{"summary":"Worker unavailable"}}
  ]
}"#;
const RESOLVED_GROUP: &str = r#"{
  "version":"4","groupKey":"{}:{severity=\"critical\"}","status":"resolved",
  "receiver":"opswarden","alerts":[]
}"#;

fn request(connection_id: Uuid, token: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/webhooks/alertmanager/{connection_id}"))
        .header("Content-Type", "application/json");
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_alertmanager(ctx: &common::TestContext) -> (Uuid, ServiceConnection) {
    let team_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "alertmanager", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(connection.id, CredentialKind::WebhookSigningSecret, TOKEN)
        .await
        .unwrap();

    let mut rule = AutomationRule::new(
        team_id,
        "Critical Alertmanager group",
        connection.id,
        "alert_firing",
        json!({"severity": "critical", "receiver": "opswarden"}),
        "create_incident",
        None,
        json!({"severity": "critical", "title": "Alertmanager group for {{receiver}}"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (team_id, connection)
}

#[tokio::test]
async fn firing_group_creates_one_incident_and_exact_retry_is_a_noop() {
    let ctx = test_context();
    let (team_id, connection) = seed_alertmanager(&ctx).await;

    let first = ctx
        .app
        .clone()
        .oneshot(request(connection.id, Some(TOKEN), FIRING_GROUP))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first = json_body(first).await;
    assert_eq!(first["duplicate"], false);
    assert_eq!(first["rules_triggered"], 1, "receipt: {first}");
    assert_eq!(first["rules_failed"], 0, "receipt: {first}");

    let retry = ctx
        .app
        .clone()
        .oneshot(request(connection.id, Some(TOKEN), FIRING_GROUP))
        .await
        .unwrap();
    assert_eq!(retry.status(), StatusCode::ACCEPTED);
    let retry = json_body(retry).await;
    assert_eq!(retry["duplicate"], true);
    assert_eq!(retry["rules_triggered"], 0);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].title, "Alertmanager group for opswarden");
    assert_eq!(incidents[0].severity, Severity::Critical);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
}

#[tokio::test]
async fn missing_or_wrong_bearer_token_is_rejected_without_reserving_delivery() {
    let ctx = test_context();
    let (_, connection) = seed_alertmanager(&ctx).await;

    let missing = ctx
        .app
        .clone()
        .oneshot(request(connection.id, None, FIRING_GROUP))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

    let wrong = ctx
        .app
        .clone()
        .oneshot(request(connection.id, Some("wrong"), FIRING_GROUP))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(ctx.webhook_deliveries.all().is_empty());
}

#[tokio::test]
async fn malformed_json_and_wrong_content_type_are_rejected() {
    let ctx = test_context();
    let (_, connection) = seed_alertmanager(&ctx).await;

    let malformed = ctx
        .app
        .clone()
        .oneshot(request(connection.id, Some(TOKEN), "not-json"))
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let wrong_type = Request::builder()
        .method("POST")
        .uri(format!("/webhooks/alertmanager/{}", connection.id))
        .header("Content-Type", "text/plain")
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body(Body::from(FIRING_GROUP))
        .unwrap();
    let response = ctx.app.clone().oneshot(wrong_type).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(ctx.webhook_deliveries.all().is_empty());
}

#[tokio::test]
async fn resolved_group_is_accepted_and_durably_ignored() {
    let ctx = test_context();
    let (team_id, connection) = seed_alertmanager(&ctx).await;

    let response = ctx
        .app
        .clone()
        .oneshot(request(connection.id, Some(TOKEN), RESOLVED_GROUP))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["duplicate"], false);
    assert_eq!(receipt["rules_triggered"], 0);
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap()
        .is_empty());
    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, WebhookDeliveryStatus::Ignored);
}
