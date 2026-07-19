mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{test_context, TestContext};
use opswarden_server::domain::incident::{Incident, Severity};
use opswarden_server::domain::team::Role;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

/// The authenticated test user is the nil UUID (see the dummy token service).
fn me() -> Uuid {
    Uuid::nil()
}

async fn send(
    ctx: &TestContext,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", "Bearer mock_jwt_token");
    let request = if let Some(b) = body {
        builder = builder.header("Content-Type", "application/json");
        builder.body(Body::from(b.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let response = ctx.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn create_list_and_get_a_release() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    let (status, created) = send(
        &ctx,
        "POST",
        "/api/releases",
        Some(json!({ "team_id": team_id, "title": "v1.0.0", "steps": ["build", "prod"] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["state"], "created");
    assert_eq!(created["steps"].as_array().unwrap().len(), 2);
    assert_eq!(created["steps"][0]["position"], 0);
    assert_eq!(created["steps"][1]["position"], 1);
    assert!(created["updated_at"].is_string());
    let release_id = created["release_id"].as_str().unwrap();

    let (status, list) = send(
        &ctx,
        "GET",
        &format!("/api/releases?team_id={team_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["progress"], json!({ "completed": 0, "total": 2 }));
    assert_eq!(
        list[0]["next_step"],
        json!({ "position": 0, "name": "build" })
    );
    assert_eq!(list[0]["blockers"], json!([]));
    assert!(list[0]["updated_at"].is_string());

    let (status, got) = send(&ctx, "GET", &format!("/api/releases/{release_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["title"], "v1.0.0");
}

#[tokio::test]
async fn steps_validate_in_order_and_reject_out_of_order() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    let (_s, created) = send(
        &ctx,
        "POST",
        "/api/releases",
        Some(json!({ "team_id": team_id, "title": "v1", "steps": ["build", "prod"] })),
    )
    .await;
    let release_id = created["release_id"].as_str().unwrap().to_string();

    // Out-of-order first step is refused.
    let (status, _b) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/steps/prod/validate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Validating in order promotes to in_progress.
    let (status, validated) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/steps/build/validate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(validated["state"], "in_progress");
    assert_eq!(validated["steps"][0]["validated"], true);
}

#[tokio::test]
async fn linking_an_active_incident_blocks_then_resolving_unblocks() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    // An in-progress release.
    let (_s, created) = send(
        &ctx,
        "POST",
        "/api/releases",
        Some(json!({ "team_id": team_id, "title": "v2", "steps": ["build", "prod"] })),
    )
    .await;
    let release_id = created["release_id"].as_str().unwrap().to_string();
    send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/steps/build/validate"),
        None,
    )
    .await;

    // An active (acknowledged) incident in the same team.
    let mut incident = Incident::new(team_id, "prod is down", Severity::Critical).unwrap();
    incident.acknowledge().unwrap();
    let incident_id = incident.id;
    ctx.incidents.seed_incident(incident);

    // Linking it blocks the release.
    let (status, linked) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/incidents/{incident_id}/link"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(linked["state"], "blocked");
    assert_eq!(linked["linked_incident_ids"].as_array().unwrap().len(), 1);

    let (status, list) = send(
        &ctx,
        "GET",
        &format!("/api/releases?team_id={team_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list[0]["state"], "blocked");
    assert_eq!(list[0]["progress"], json!({ "completed": 1, "total": 2 }));
    assert_eq!(
        list[0]["next_step"],
        json!({ "position": 1, "name": "prod" })
    );
    assert_eq!(
        list[0]["blockers"][0]["incident_id"],
        incident_id.to_string()
    );
    assert_eq!(list[0]["blockers"][0]["title"], "prod is down");
    assert_eq!(list[0]["blockers"][0]["severity"], "critical");

    // While blocked, step validation is refused.
    let (status, _b) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/steps/prod/validate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Resolving the incident auto-unblocks the release.
    let (status, _b) = send(
        &ctx,
        "PUT",
        &format!("/api/incidents/{incident_id}/status"),
        Some(json!({ "status": "resolved" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, got) = send(&ctx, "GET", &format!("/api/releases/{release_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["state"], "in_progress");

    // And now the previously-blocked step validates.
    let (status, done) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/steps/prod/validate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(done["state"], "completed");
}

#[tokio::test]
async fn manager_can_cancel_a_release() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    let (_s, created) = send(
        &ctx,
        "POST",
        "/api/releases",
        Some(json!({ "team_id": team_id, "title": "v3", "steps": ["build"] })),
    )
    .await;
    let release_id = created["release_id"].as_str().unwrap().to_string();

    let (status, cancelled) = send(
        &ctx,
        "POST",
        &format!("/api/releases/{release_id}/cancel"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["state"], "cancelled");
}

#[tokio::test]
async fn creating_a_release_without_auth_is_unauthorized() {
    let ctx = test_context();
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "team_id": Uuid::new_v4(), "title": "x", "steps": ["a"] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
