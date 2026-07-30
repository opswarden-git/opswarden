mod common;

use axum::{body::Body, http::Request, http::StatusCode, response::Response};
use common::test_context;
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection, WebhookDeliveryStatus,
};
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "alertmanager-secret";
const STARTS_AT: &str = "2026-07-30T12:00:00Z";
const ENDS_AT: &str = "2026-07-30T12:05:00Z";

fn alert(status: &str, fingerprint: &str, starts_at: &str, ends_at: &str) -> Value {
    json!({
        "status": status,
        "fingerprint": fingerprint,
        "startsAt": starts_at,
        "endsAt": ends_at,
        "generatorURL": "https://prometheus.example/graph",
        "labels": {
            "alertname": format!("{fingerprint}Down"),
            "severity": "critical",
            "instance": fingerprint
        },
        "annotations": {
            "summary": format!("{fingerprint} unavailable"),
            "description": "Health probe failed"
        }
    })
}

fn group(alerts: Vec<Value>) -> String {
    json!({
        "version": "4",
        "groupKey": "{}:{severity=\"critical\"}",
        "status": "firing",
        "receiver": "opswarden",
        "commonLabels": {"severity": "critical"},
        "alerts": alerts
    })
    .to_string()
}

fn request(connection_id: Uuid, token: Option<&str>, body: impl Into<Body>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/webhooks/alertmanager/{connection_id}"))
        .header("Content-Type", "application/json");
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    builder.body(body.into()).unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn metrics(ctx: &common::TestContext) -> String {
    let response = ctx
        .app
        .clone()
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn connection(
    ctx: &common::TestContext,
    team_id: Uuid,
    service: &str,
    token: &str,
) -> ServiceConnection {
    let connection = ServiceConnection::new(team_id, service, Uuid::new_v4()).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(connection.id, CredentialKind::WebhookSigningSecret, token)
        .await
        .unwrap();
    connection
}

async fn rule(
    ctx: &common::TestContext,
    team_id: Uuid,
    trigger: &ServiceConnection,
    kind: &str,
    enabled: bool,
    reaction: &str,
    reaction_connection_id: Option<Uuid>,
) {
    let mut rule = AutomationRule::new(
        team_id,
        format!("Alertmanager {kind}"),
        trigger.id,
        kind,
        json!({"severity": "critical", "receiver": "opswarden"}),
        reaction,
        reaction_connection_id,
        if reaction == "http_notify" {
            json!({"message": "{{alertname}} on {{instance}}"})
        } else {
            json!({"severity": "critical", "title": "{{alertname}}: {{summary}}"})
        },
        Uuid::new_v4(),
    )
    .unwrap();
    rule.set_enabled(enabled);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
}

async fn seed_lifecycle(ctx: &common::TestContext, team_id: Uuid) -> ServiceConnection {
    let connection = connection(ctx, team_id, "alertmanager", TOKEN).await;
    rule(
        ctx,
        team_id,
        &connection,
        "alert_firing",
        true,
        "create_incident",
        None,
    )
    .await;
    rule(
        ctx,
        team_id,
        &connection,
        "alert_resolved",
        true,
        "create_incident",
        None,
    )
    .await;
    connection
}

#[tokio::test]
async fn mixed_group_creates_one_durable_event_per_alert_lifecycle_transition() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let alertmanager = seed_lifecycle(&ctx, team_id).await;
    let payload = group(vec![
        alert("firing", "api", STARTS_AT, ENDS_AT),
        alert("resolved", "worker", STARTS_AT, ENDS_AT),
    ]);

    let response = ctx
        .app
        .clone()
        .oneshot(request(alertmanager.id, Some(TOKEN), payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["transitions_received"], 2);
    assert_eq!(receipt["transitions_duplicate"], 0);
    assert_eq!(receipt["rules_triggered"], 2);
    assert_eq!(receipt["rules_failed"], 0);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(ctx.webhook_deliveries.all().len(), 2);
    assert!(ctx
        .webhook_deliveries
        .all()
        .iter()
        .all(|delivery| delivery.status == WebhookDeliveryStatus::Processed));
    assert!(ctx
        .automation_runs
        .all()
        .iter()
        .all(|run| run.status == AutomationRunStatus::Succeeded));
    assert!(metrics(&ctx).await.contains(r#"outcome="accepted"} 2"#));
}

#[tokio::test]
async fn semantic_retry_is_duplicate_but_new_lifecycle_is_accepted() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let alertmanager = seed_lifecycle(&ctx, team_id).await;
    let first = group(vec![alert("firing", "api", STARTS_AT, ENDS_AT)]);
    let retry = format!(
        r#"{{"alerts":[{{"endsAt":"2030-01-01T00:00:00Z","startsAt":"{STARTS_AT}","fingerprint":"api","status":"firing","labels":{{"severity":"critical","alertname":"apiDown"}}}}],"receiver":"opswarden","status":"firing","groupKey":"{{}}:{{severity=\"critical\"}}"}}"#
    );

    for (payload, duplicate) in [
        (first, false),
        (retry, true),
        (
            group(vec![alert("resolved", "api", STARTS_AT, ENDS_AT)]),
            false,
        ),
        (
            group(vec![alert(
                "firing",
                "api",
                "2026-07-31T12:00:00Z",
                ENDS_AT,
            )]),
            false,
        ),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(request(alertmanager.id, Some(TOKEN), payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(json_body(response).await["duplicate"], duplicate);
    }
    assert_eq!(ctx.webhook_deliveries.all().len(), 3);
    assert!(metrics(&ctx).await.contains(r#"outcome="duplicate"} 1"#));
}

#[tokio::test]
async fn authentication_content_type_connection_and_one_mib_limit_are_enforced() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let alertmanager = seed_lifecycle(&ctx, team_id).await;
    let payload = group(vec![alert("firing", "api", STARTS_AT, ENDS_AT)]);

    let missing = ctx
        .app
        .clone()
        .oneshot(request(alertmanager.id, None, payload.clone()))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
    let wrong = ctx
        .app
        .clone()
        .oneshot(request(alertmanager.id, Some("wrong"), payload.clone()))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    let wrong_type = Request::post(format!("/webhooks/alertmanager/{}", alertmanager.id))
        .header("Content-Type", "text/plain")
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body(Body::from(payload.clone()))
        .unwrap();
    assert_eq!(
        ctx.app.clone().oneshot(wrong_type).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let github = connection(&ctx, team_id, "github", TOKEN).await;
    assert_eq!(
        ctx.app
            .clone()
            .oneshot(request(github.id, Some(TOKEN), payload))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let oversized = vec![b'x'; 1024 * 1024 + 1];
    assert_eq!(
        ctx.app
            .clone()
            .oneshot(request(alertmanager.id, Some(TOKEN), oversized))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert!(ctx.webhook_deliveries.all().is_empty());
    assert!(metrics(&ctx).await.contains(r#"outcome="rejected"} 5"#));
}

#[tokio::test]
async fn teams_and_disabled_rules_are_isolated_and_ignored() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    let connection_a = connection(&ctx, team_a, "alertmanager", TOKEN).await;
    let connection_b = connection(&ctx, team_b, "alertmanager", TOKEN).await;
    rule(
        &ctx,
        team_a,
        &connection_a,
        "alert_firing",
        false,
        "create_incident",
        None,
    )
    .await;
    rule(
        &ctx,
        team_b,
        &connection_b,
        "alert_firing",
        true,
        "create_incident",
        None,
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(request(
            connection_a.id,
            Some(TOKEN),
            group(vec![alert("firing", "api", STARTS_AT, ENDS_AT)]),
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["transitions_ignored"], 1);
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_a)
        .await
        .unwrap()
        .is_empty());
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_b)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        ctx.webhook_deliveries.all()[0].status,
        WebhookDeliveryStatus::Ignored
    );
    assert!(metrics(&ctx).await.contains(r#"outcome="ignored"} 1"#));
}

#[tokio::test]
async fn reaction_failure_is_reported_and_persisted() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let trigger = connection(&ctx, team_id, "alertmanager", TOKEN).await;
    let http = connection(&ctx, team_id, "http", "unused").await;
    ctx.connection_credentials
        .store_credential(
            http.id,
            CredentialKind::EndpointUrl,
            "https://hooks.example.com/alertmanager",
        )
        .await
        .unwrap();
    rule(
        &ctx,
        team_id,
        &trigger,
        "alert_firing",
        true,
        "http_notify",
        Some(http.id),
    )
    .await;
    ctx.notifier.fail_requests();

    let response = ctx
        .app
        .clone()
        .oneshot(request(
            trigger.id,
            Some(TOKEN),
            group(vec![alert("firing", "api", STARTS_AT, ENDS_AT)]),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["rules_failed"], 1);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Failed);
    assert_eq!(runs[0].error_code.as_deref(), Some("reaction_http_5xx"));
    let delivery = &ctx.webhook_deliveries.all()[0];
    assert_eq!(delivery.status, WebhookDeliveryStatus::Processed);
    assert!(metrics(&ctx).await.contains(r#"outcome="failed"} 1"#));
}
