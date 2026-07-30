mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::team::Role;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

/// The authenticated test user is the nil UUID (see the dummy token service).
fn me() -> Uuid {
    Uuid::nil()
}

// ---------------------------------------------------------------------------
// POST /api/teams/{team_id}/channels
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_channel_returns_201() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "général" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["name"], "général");
    assert_eq!(body["team_id"], team_id.to_string());
    assert!(body["id"].as_str().is_some());

    let stored = ctx.channels.all_for_team(team_id);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].name, "général");
}

#[tokio::test]
async fn create_channel_trims_whitespace() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "  ops  " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let stored = ctx.channels.all_for_team(team_id);
    assert_eq!(stored[0].name, "ops");
}

#[tokio::test]
async fn create_channel_with_blank_name_returns_400() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "   " }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "invalid_channel_name");
}

#[tokio::test]
async fn create_channel_duplicate_name_returns_400() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    // First creation succeeds.
    let _ = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "alerts" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Duplicate name in the same team fails.
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "alerts" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_channel_observer_is_forbidden() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "ops" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_channel_unauthenticated_is_401() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "ops" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// GET /api/teams/{team_id}/channels
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_channels_returns_empty_array_when_none() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_channels_returns_created_channels() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    // Create two channels.
    for name in ["général", "alerts"] {
        let _ = ctx
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/teams/{team_id}/channels"))
                    .header("Authorization", "Bearer mock_jwt_token")
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "name": name }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let channels = body.as_array().unwrap();
    assert_eq!(channels.len(), 2);
}

#[tokio::test]
async fn list_channels_observer_is_forbidden() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// DELETE /api/teams/{team_id}/channels/{channel_id}
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_channel_returns_204() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    // Create a channel first.
    let create_response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "to-delete" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let channel_id = created["id"].as_str().unwrap();

    // Delete it.
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/channels/{channel_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(ctx.channels.all_for_team(team_id).is_empty());
}

#[tokio::test]
async fn delete_channel_not_found_returns_404() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Responder);

    let fake_channel_id = Uuid::new_v4();
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/channels/{fake_channel_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "channel_not_found");
}

#[tokio::test]
async fn delete_channel_observer_is_forbidden() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/channels/{}", Uuid::new_v4()))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn manager_can_create_and_delete_channels() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, me(), Role::Manager);

    // Create
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/channels"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "name": "manager-channel" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let channel_id = created["id"].as_str().unwrap();

    // Delete
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/channels/{channel_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
