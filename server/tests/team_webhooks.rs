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
use opswarden_server::domain::incident::{Incident, IncidentStatus, Severity};
use opswarden_server::domain::release::{Release, ReleaseState};
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ReleaseRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET_A: &str = "team-a-signing-secret";
const SECRET_B: &str = "team-b-signing-secret";
const GITLAB_TOKEN: &str = "team-gitlab-token";
const GENERIC_TOKEN: &str = "team-generic-token";
const GENERIC_EVENT: &str = r#"{
    "source":"jury",
    "title":"Production deployment failed",
    "message":"Health check timed out",
    "severity":"critical",
    "external_id":"deploy-42",
    "event_url":"https://example.test/deployments/42",
    "ignored":{"token":"must-not-be-normalized"}
}"#;
const FAILED_RUN: &str = r#"{
    "repository":{"full_name":"opswarden/app"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"failure",
        "html_url":"https://github.com/opswarden/app/actions/runs/42"
    }
}"#;
const SUCCEEDED_RUN: &str = r#"{
    "repository":{"full_name":"opswarden/app"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"success",
        "html_url":"https://github.com/opswarden/app/actions/runs/43"
    }
}"#;
const NEW_TAG: &str = r#"{
    "ref":"refs/tags/v1.2.3",
    "created":true,
    "deleted":false,
    "after":"abcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "compare":"https://github.com/opswarden/app/compare/v1.2.3",
    "repository":{"full_name":"opswarden/app"},
    "sender":{"login":"octocat"}
}"#;
const MERGED_PULL_REQUEST: &str = r#"{
    "action":"closed",
    "number":42,
    "repository":{"full_name":"opswarden/app"},
    "pull_request":{
        "merged":true,
        "title":"Ship VIGIL",
        "html_url":"https://github.com/opswarden/app/pull/42",
        "base":{"ref":"main"},
        "head":{"ref":"feature/vigil"},
        "merged_by":{"login":"octocat"}
    }
}"#;
const GITLAB_FAILED_PIPELINE: &str = r#"{
    "object_kind":"pipeline",
    "object_attributes":{"status":"failed","ref":"main","name":"CI","url":"https://gitlab.com/opswarden/app/-/pipelines/42"},
    "project":{"path_with_namespace":"opswarden/app"}
}"#;
const GITLAB_SUCCEEDED_PIPELINE: &str = r#"{
    "object_kind":"pipeline",
    "object_attributes":{"status":"success","ref":"main","name":"CI","url":"https://gitlab.com/opswarden/app/-/pipelines/43"},
    "project":{"path_with_namespace":"opswarden/app"}
}"#;
const GITLAB_NEW_TAG: &str = r#"{
    "object_kind":"tag_push",
    "ref":"refs/tags/v1.2.3",
    "before":"0000000000000000000000000000000000000000",
    "after":"abcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "user_username":"octocat",
    "project":{"path_with_namespace":"opswarden/app","web_url":"https://gitlab.com/opswarden/app"}
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

fn gitlab_webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    token: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/gitlab/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-Gitlab-Event-UUID", delivery_id)
        .header("X-Gitlab-Event", event)
        .header("X-Gitlab-Token", token)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn generic_webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    token: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/generic/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-OpsWarden-Delivery", delivery_id)
        .header("X-OpsWarden-Event", event)
        .header("X-OpsWarden-Token", token)
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
        json!({"message": "Alert: {{workflow}} / {{repository}}"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (github, http, rule)
}

async fn seed_github_action(
    ctx: &common::TestContext,
    team_id: Uuid,
    trigger_kind: &str,
    trigger_config: Value,
    reaction_config: Value,
) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            SECRET_A,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitHub {trigger_kind}"),
        connection.id,
        trigger_kind,
        trigger_config,
        "vigil_create_incident",
        None,
        reaction_config,
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}

async fn seed_github_reaction(
    ctx: &common::TestContext,
    team_id: Uuid,
    reaction_kind: &str,
    reaction_config: Value,
) -> (ServiceConnection, AutomationRule) {
    let actor = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", actor).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            SECRET_A,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitHub to {reaction_kind}"),
        connection.id,
        "ci_failed",
        json!({}),
        reaction_kind,
        None,
        reaction_config,
        actor,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (connection, rule)
}

async fn seed_gitlab_action(
    ctx: &common::TestContext,
    team_id: Uuid,
    trigger_kind: &str,
    trigger_config: Value,
    reaction_config: Value,
) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "gitlab", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            GITLAB_TOKEN,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitLab {trigger_kind}"),
        connection.id,
        trigger_kind,
        trigger_config,
        "vigil_create_incident",
        None,
        reaction_config,
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}

async fn seed_generic_action(ctx: &common::TestContext, team_id: Uuid) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "generic", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            GENERIC_TOKEN,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "Generic deployment failure",
        connection.id,
        "generic_event",
        json!({
            "event_type": "deployment_failed",
            "source": "jury",
            "severity": "critical"
        }),
        "vigil_create_incident",
        None,
        json!({
            "severity": "critical",
            "title": "{{source}}: {{title}} ({{external_id}})"
        }),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}

#[tokio::test]
async fn signed_delivery_creates_incident_and_durable_run_then_duplicate_is_noop() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (connection, mut rule) = seed_automation(
        &ctx,
        team_id,
        SECRET_A,
        json!({"repository": "opswarden/app", "branch": "main"}),
        "vigil_create_incident",
    )
    .await;
    let mut definition = rule.definition();
    definition.reaction_config = json!({
        "severity": "critical",
        "title": "[{{repository}}] {{workflow}} failed"
    });
    rule.replace_definition(definition).unwrap();
    assert!(ctx.automation_rules.update_rule(&rule).await.unwrap());
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
    assert_eq!(incidents[0].title, "[opswarden/app] CI failed");
    assert!(incidents[0].description.contains("Branch: main"));
    assert_eq!(incidents[0].severity.to_string(), "critical");

    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, WebhookDeliveryStatus::Processed);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(runs[0].incident_id, Some(incidents[0].id));
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "incident_created",
            "incident_id": incidents[0].id,
            "severity": "critical",
        })
    );
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "rule_triggered",
            "service": "github",
            "rule_name": "GitHub CI failed",
            "result": "incident_created",
            "incident_id": incidents[0].id,
        })
    );

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
async fn extended_github_actions_run_end_to_end_with_filters_and_templates() {
    let cases = [
        (
            "ci_succeeded",
            "workflow_run",
            SUCCEEDED_RUN,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "success"}),
            json!({"severity": "medium", "title": "{{workflow}} succeeded on {{repository}}"}),
            "CI succeeded on opswarden/app",
        ),
        (
            "tag_pushed",
            "push",
            NEW_TAG,
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            json!({"severity": "medium", "title": "Tag {{tag}} pushed by {{actor}}"}),
            "Tag v1.2.3 pushed by octocat",
        ),
        (
            "pr_merged",
            "pull_request",
            MERGED_PULL_REQUEST,
            json!({"repository": "opswarden/app", "branch": "main", "source_branch": "feature/vigil"}),
            json!({"severity": "medium", "title": "PR #{{pull_request_number}} {{pull_request_title}}"}),
            "PR #42 Ship VIGIL",
        ),
    ];

    for (index, (kind, provider_event, body, trigger_config, reaction_config, expected_title)) in
        cases.into_iter().enumerate()
    {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        let connection =
            seed_github_action(&ctx, team_id, kind, trigger_config, reaction_config).await;
        let response = ctx
            .app
            .clone()
            .oneshot(webhook_request(
                connection.id,
                &format!("extended-{index}"),
                provider_event,
                SECRET_A,
                body,
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
        assert_eq!(incidents[0].title, expected_title);
        assert_eq!(
            ctx.webhook_deliveries.all()[0].status,
            WebhookDeliveryStatus::Processed
        );
        assert_eq!(
            ctx.automation_runs.all()[0].status,
            AutomationRunStatus::Succeeded
        );
    }
}

#[tokio::test]
async fn gitlab_actions_run_end_to_end_with_token_filters_templates_and_deduplication() {
    let cases = [
        (
            "ci_failed",
            "Pipeline Hook",
            GITLAB_FAILED_PIPELINE,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "failed"}),
            json!({"severity": "high", "title": "{{workflow}} failed on {{repository}}"}),
            "CI failed on opswarden/app",
        ),
        (
            "ci_succeeded",
            "Pipeline Hook",
            GITLAB_SUCCEEDED_PIPELINE,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "success"}),
            json!({"severity": "medium", "title": "{{workflow}} succeeded on {{repository}}"}),
            "CI succeeded on opswarden/app",
        ),
        (
            "tag_pushed",
            "Tag Push Hook",
            GITLAB_NEW_TAG,
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            json!({"severity": "medium", "title": "Tag {{tag}} pushed by {{actor}}"}),
            "Tag v1.2.3 pushed by octocat",
        ),
    ];

    for (index, (kind, provider_event, body, trigger_config, reaction_config, expected_title)) in
        cases.into_iter().enumerate()
    {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        let connection =
            seed_gitlab_action(&ctx, team_id, kind, trigger_config, reaction_config).await;
        let delivery_id = format!("gitlab-{index}");
        let response = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &delivery_id,
                provider_event,
                GITLAB_TOKEN,
                body,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let receipt = json_body(response).await;
        assert_eq!(receipt["rules_triggered"], 1);
        assert_eq!(receipt["rules_failed"], 0);
        let incidents = ctx
            .incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].title, expected_title);
        assert_eq!(
            ctx.automation_runs.all()[0].status,
            AutomationRunStatus::Succeeded
        );

        let duplicate = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &delivery_id,
                provider_event,
                GITLAB_TOKEN,
                body,
            ))
            .await
            .unwrap();
        assert_eq!(json_body(duplicate).await["duplicate"], true);
        assert_eq!(ctx.automation_runs.all().len(), 1);

        let rejected = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &format!("wrong-token-{index}"),
                provider_event,
                "wrong-token",
                body,
            ))
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(ctx.automation_runs.all().len(), 1);
    }
}

#[tokio::test]
async fn generic_json_runs_through_auth_filters_templates_durability_and_deduplication() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let connection = seed_generic_action(&ctx, team_id).await;

    let accepted = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-42",
            "deployment_failed",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    let receipt = json_body(accepted).await;
    assert_eq!(receipt["duplicate"], false);
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(
        incidents[0].title,
        "jury: Production deployment failed (deploy-42)"
    );
    assert_eq!(incidents[0].severity, Severity::Critical);
    assert!(incidents[0]
        .description
        .contains("Event type: deployment_failed"));
    assert!(incidents[0]
        .description
        .contains("Message: Health check timed out"));
    assert!(!incidents[0].description.contains("must-not-be-normalized"));
    assert_eq!(ctx.automation_runs.all().len(), 1);
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
    assert_eq!(ctx.webhook_deliveries.all().len(), 1);
    assert_eq!(
        ctx.webhook_deliveries.all()[0].status,
        WebhookDeliveryStatus::Processed
    );

    let duplicate = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-42",
            "deployment_failed",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(duplicate).await["duplicate"], true);
    assert_eq!(ctx.automation_runs.all().len(), 1);

    let filtered = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-43",
            "deployment_succeeded",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(filtered).await["rules_triggered"], 0);
    assert_eq!(ctx.automation_runs.all().len(), 1);
    assert_eq!(ctx.webhook_deliveries.all().len(), 2);
    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(
        deliveries
            .iter()
            .find(|delivery| delivery.provider_delivery_id == "generic-delivery-43")
            .unwrap()
            .status,
        WebhookDeliveryStatus::Ignored
    );

    let rejected = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-44",
            "deployment_failed",
            "wrong-token",
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(ctx.webhook_deliveries.all().len(), 2);
}

#[tokio::test]
async fn generic_endpoint_rejects_missing_headers_content_type_and_invalid_or_large_json() {
    let ctx = test_context();
    let connection_id = Uuid::new_v4();
    for request in [
        Request::builder()
            .method("POST")
            .uri(format!("/webhooks/generic/{connection_id}"))
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .unwrap(),
        Request::builder()
            .method("POST")
            .uri(format!("/webhooks/generic/{connection_id}"))
            .header("X-OpsWarden-Delivery", "delivery")
            .header("X-OpsWarden-Event", "event")
            .header("X-OpsWarden-Token", "token")
            .body(Body::from("{}"))
            .unwrap(),
        generic_webhook_request(connection_id, "delivery", "event", "token", "not-json"),
        generic_webhook_request(connection_id, "delivery", "event", "token", "[]"),
        generic_webhook_request(
            connection_id,
            "delivery",
            "event",
            "token",
            r#"{"severity":"urgent"}"#,
        ),
    ] {
        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let oversized = format!(r#"{{"ignored":"{}"}}"#, "x".repeat(64 * 1024));
    let response = ctx
        .app
        .oneshot(generic_webhook_request(
            connection_id,
            "large",
            "event",
            "token",
            &oversized,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
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
    assert_eq!(ctx.notifier.calls()[0].1, "Alert: CI / opswarden/app");
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
    let (tx, mut rx) = mpsc::unbounded_channel();
    ctx.events
        .register(Uuid::new_v4(), HashSet::from([failing_team]), tx);
    while rx.try_recv().is_ok() {}
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
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "rule_failed",
            "service": "github",
            "rule_name": "GitHub CI failed",
            "error": "invalid_automation_rule",
        })
    );
    assert!(ctx
        .incidents
        .list_incidents_for_team(failing_team)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn github_event_validates_the_next_release_step_and_records_the_run() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let release =
        Release::new(team_id, "v1.1.0", vec!["build".into(), "production".into()]).unwrap();
    ctx.releases.save_release(&release).await.unwrap();
    let (connection, rule) = seed_github_reaction(
        &ctx,
        team_id,
        "vigil_validate_release_step",
        json!({"release_id": release.id, "step": "build"}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "validate-release-step",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let stored = ctx
        .releases
        .find_release_by_id(release.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base_state, ReleaseState::InProgress);
    assert_eq!(stored.steps[0].validated_by, rule.created_by);
    assert!(stored.steps[0].validated_at.is_some());
    assert!(stored.steps[1].validated_at.is_none());
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
}

#[tokio::test]
async fn github_event_blocks_an_in_progress_release_with_a_linked_incident() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let mut release =
        Release::new(team_id, "v1.1.0", vec!["build".into(), "production".into()]).unwrap();
    release
        .validate_step("build", Uuid::new_v4(), false)
        .unwrap();
    ctx.releases.save_release(&release).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        team_id,
        "vigil_block_release",
        json!({
            "release_id": release.id,
            "severity": "critical",
            "title": "{{workflow}} blocks release"
        }),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "block-release",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let linked = ctx
        .releases
        .list_linked_incident_ids(release.id)
        .await
        .unwrap();
    assert_eq!(linked.len(), 1);
    let incident = ctx
        .incidents
        .find_incident_by_id(linked[0])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(incident.title, "CI blocks release");
    assert_eq!(incident.severity, Severity::Critical);
    assert_eq!(
        release.effective_state(
            ctx.releases
                .count_active_linked_incidents(release.id)
                .await
                .unwrap()
                > 0
        ),
        ReleaseState::Blocked
    );
    let run = &ctx.automation_runs.all()[0];
    assert_eq!(run.status, AutomationRunStatus::Succeeded);
    assert_eq!(run.incident_id, Some(incident.id));
}

#[tokio::test]
async fn github_event_escalates_an_acknowledged_incident_with_an_audit_event() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let mut incident = Incident::new(team_id, "Database latency", Severity::High).unwrap();
    incident.acknowledge().unwrap();
    ctx.incidents.save_incident(&incident).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        team_id,
        "vigil_escalate_incident",
        json!({"incident_id": incident.id}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "escalate-incident",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let stored = ctx
        .incidents
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, IncidentStatus::Escalated);
    let events = ctx
        .incidents
        .list_events_for_incident(incident.id, 10)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
}

#[tokio::test]
async fn native_reactions_cannot_mutate_another_teams_release() {
    let ctx = test_context();
    let source_team = Uuid::new_v4();
    let foreign_release = Release::new(
        Uuid::new_v4(),
        "foreign",
        vec!["build".into(), "production".into()],
    )
    .unwrap();
    ctx.releases.save_release(&foreign_release).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        source_team,
        "vigil_validate_release_step",
        json!({"release_id": foreign_release.id, "step": "build"}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "foreign-release",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["rules_failed"], 1);
    let stored = ctx
        .releases
        .find_release_by_id(foreign_release.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base_state, ReleaseState::Created);
    assert!(stored.steps.iter().all(|step| !step.is_validated()));
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Failed
    );
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
