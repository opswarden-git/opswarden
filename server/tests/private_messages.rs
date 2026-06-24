mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use common::test_context;
use opswarden_server::domain::private_message::PrivateMessage;
use opswarden_server::domain::team::Role;
use opswarden_server::domain::user::{Email, User};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

/// The authenticated test user is the nil UUID (see the dummy token service).
fn me() -> Uuid {
    Uuid::nil()
}

fn seed_user(ctx: &common::TestContext, id: Uuid) {
    ctx.users.seed_user(User {
        id,
        email: Email::new(format!("user-{id}@test.com")).unwrap(),
        password_hash: "hash".to_string(),
        created_at: Utc::now(),
    });
}

#[tokio::test]
async fn send_persists_and_returns_201() {
    let ctx = test_context();
    let recipient = Uuid::new_v4();
    let team = Uuid::new_v4();
    seed_user(&ctx, recipient);
    ctx.teams.seed_member(team, me(), Role::Observer);
    ctx.teams.seed_member(team, recipient, Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/private-messages")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "recipient_id": recipient, "content": "ping" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let messages = ctx.private_messages.all();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "ping");
    assert_eq!(messages[0].sender_id, me());
    assert_eq!(messages[0].recipient_id, recipient);
}

#[tokio::test]
async fn send_to_a_non_shared_team_member_is_forbidden() {
    let ctx = test_context();
    let recipient = Uuid::new_v4();
    // Recipient exists but is in a different team; the sender shares none.
    seed_user(&ctx, recipient);
    ctx.teams
        .seed_member(Uuid::new_v4(), recipient, Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/private-messages")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "recipient_id": recipient, "content": "hello?" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "no_shared_team");
    assert!(ctx.private_messages.all().is_empty());
}

#[tokio::test]
async fn send_without_auth_is_unauthorized() {
    let ctx = test_context();

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/private-messages")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "recipient_id": Uuid::new_v4(), "content": "hi" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_returns_the_conversation_for_a_participant() {
    let ctx = test_context();
    let peer = Uuid::new_v4();
    let team = Uuid::new_v4();
    seed_user(&ctx, peer);
    ctx.teams.seed_member(team, me(), Role::Observer);
    ctx.teams.seed_member(team, peer, Role::Observer);
    ctx.private_messages
        .seed(PrivateMessage::new(me(), peer, "from me").unwrap());
    ctx.private_messages
        .seed(PrivateMessage::new(peer, me(), "from peer").unwrap());

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/private-messages?peer_id={peer}"))
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
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn list_for_a_non_shared_peer_is_forbidden() {
    let ctx = test_context();
    let peer = Uuid::new_v4();
    // Peer exists but shares no team with the requester.
    seed_user(&ctx, peer);
    ctx.teams.seed_member(Uuid::new_v4(), peer, Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/private-messages?peer_id={peer}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
