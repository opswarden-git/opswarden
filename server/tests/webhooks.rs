mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context_with_github_rule;
use opswarden_server::adapters::crypto::hmac::hmac_sha256;
use opswarden_server::domain::incident::Severity;
use opswarden_server::ports::IncidentRepo;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET: &str = "github-webhook-secret";
const FAILED_RUN: &str = r#"{"workflow_run":{"conclusion":"failure"}}"#;

fn signature_for(body: &str) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256(SECRET.as_bytes(), body.as_bytes()))
    )
}

#[tokio::test]
async fn signed_github_ci_failure_opens_a_high_incident() {
    let team_id = Uuid::new_v4();
    let ctx = test_context_with_github_rule(team_id);
    ctx.vault.seed("github", SECRET);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/github")
                .header("X-Hub-Signature-256", signature_for(FAILED_RUN))
                .header("Content-Type", "application/json")
                .body(Body::from(FAILED_RUN.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["received"], true);
    assert_eq!(json["rules_triggered"], 1);

    let created = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].severity, Severity::High);
}

#[tokio::test]
async fn invalid_signature_is_rejected_and_creates_nothing() {
    let team_id = Uuid::new_v4();
    let ctx = test_context_with_github_rule(team_id);
    ctx.vault.seed("github", SECRET);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/github")
                .header("X-Hub-Signature-256", "sha256=deadbeef")
                .header("Content-Type", "application/json")
                .body(Body::from(FAILED_RUN.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn unconfigured_service_returns_not_found() {
    // No secret seeded into the vault -> the service is unknown.
    let ctx = test_context_with_github_rule(Uuid::new_v4());

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/github")
                .header("X-Hub-Signature-256", signature_for(FAILED_RUN))
                .body(Body::from(FAILED_RUN.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn about_exposes_the_automation_catalog() {
    let ctx = test_context_with_github_rule(Uuid::new_v4());

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/about.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let services = json["server"]["services"].as_array().unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0]["name"], "github");
    assert_eq!(services[0]["actions"][0]["name"], "ci_failed");

    // The catalog must surface both REActions the engine supports, not only
    // create_incident: the generic HTTP `notify` reaction is part of the contract.
    let reaction_names: Vec<&str> = services[0]["reactions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert!(reaction_names.contains(&"create_incident"));
    assert!(reaction_names.contains(&"notify"));
}
