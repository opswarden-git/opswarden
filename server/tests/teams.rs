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

#[tokio::test]
async fn list_members_returns_the_roster_for_a_team_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();
    let teammate = Uuid::new_v4();

    ctx.teams.seed_member(team_id, requester, Role::Manager);
    ctx.teams.seed_member(team_id, teammate, Role::Responder);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/members"))
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
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let members = json.as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert!(members
        .iter()
        .all(|m| m["email"].as_str().unwrap().contains('@')));
    assert!(members
        .iter()
        .any(|m| m["user_id"] == teammate.to_string() && m["role"] == "responder"));
}

#[tokio::test]
async fn list_members_is_forbidden_for_a_non_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    // The requester (mock token -> nil uuid) is NOT seeded into this team.
    ctx.teams
        .seed_member(team_id, Uuid::new_v4(), Role::Manager);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/members"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_members_requires_authentication() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/members"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn set_role_request(team_id: Uuid, user_id: Uuid, role: &str) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(format!("/api/teams/{team_id}/members/{user_id}/role"))
        .header("Authorization", "Bearer mock_jwt_token")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::json!({ "role": role }).to_string()))
        .unwrap()
}

#[tokio::test]
async fn manager_promotes_a_member_to_responder() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let manager = Uuid::nil();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, manager, Role::Manager);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(set_role_request(team_id, member, "responder"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Confirm the change through the roster read.
    let roster = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/members"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(roster.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let row = json
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["user_id"] == member.to_string())
        .unwrap();
    assert_eq!(row["role"], "responder");
}

#[tokio::test]
async fn set_member_role_is_forbidden_for_a_non_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Responder);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, member, "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn set_member_role_on_an_unknown_member_is_not_found() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, Uuid::new_v4(), "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn set_member_role_cannot_target_the_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let manager = Uuid::nil();
    ctx.teams.seed_member(team_id, manager, Role::Manager);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, manager, "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn set_member_role_rejects_an_invalid_role() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, member, "manager"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn leave_team_removes_member_when_not_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();

    ctx.teams.seed_member(team_id, requester, Role::Responder);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/leave"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn manager_can_delete_team() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();

    ctx.teams.seed_member(team_id, requester, Role::Manager);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn list_teams_returns_the_users_teams_with_roles() {
    let ctx = test_context();
    let team = Team::new("SRE Core").unwrap();
    ctx.teams.seed_team(team.clone());
    ctx.teams.seed_member(team.id, Uuid::nil(), Role::Manager);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/teams")
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
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let teams = json.as_array().unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["name"], "SRE Core");
    assert_eq!(teams[0]["role"], "manager");
    assert_eq!(teams[0]["invitation_code"], team.invitation_code.as_str());
}
