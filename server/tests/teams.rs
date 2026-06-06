mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::team::{Role, Team};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn create_team_returns_created_for_authenticated_user() {
    let ctx = test_context();
    let payload = serde_json::json!({
        "name": "SRE Core"
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["name"], "SRE Core");
    assert!(json["invitation_code"]
        .as_str()
        .unwrap()
        .starts_with("OPS-"));
}

#[tokio::test]
async fn join_team_uses_the_invitation_code_contract() {
    let ctx = test_context();
    let team = Team::new("Ops").unwrap();
    let invitation_code = team.invitation_code.as_str().to_string();
    ctx.teams.seed_team(team.clone());

    let payload = serde_json::json!({
        "invitation_code": invitation_code
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams/join")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        ctx.teams.role_for(team.id, Uuid::nil()),
        Some(Role::Observer)
    );
}

#[tokio::test]
async fn transfer_manager_requires_the_requester_to_be_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();
    let new_manager = Uuid::new_v4();

    ctx.teams.seed_member(team_id, requester, Role::Responder);
    ctx.teams.seed_member(team_id, new_manager, Role::Observer);

    let payload = serde_json::json!({
        "new_manager_id": new_manager
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/teams/{team_id}/manager"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
