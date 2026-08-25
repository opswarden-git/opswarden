mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use common::test_context;
use opswarden_server::domain::private_message::PrivateMessage;
use opswarden_server::domain::team::Role;
use opswarden_server::domain::user::{Email, Locale, User};
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
        locale: Locale::En,
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
    assert!(body["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature == "attach_files"));
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

#[tokio::test]
async fn attachment_round_trip_is_private_bounded_metadata_plus_download() {
    let ctx = test_context();
    let recipient = Uuid::new_v4();
    let team = Uuid::new_v4();
    seed_user(&ctx, recipient);
    ctx.teams.seed_member(team, me(), Role::Observer);
    ctx.teams.seed_member(team, recipient, Role::Observer);

    let send = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/private-messages")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "recipient_id": recipient,
                        "content": "runbook",
                        "attachments": [{
                            "file_name": "runbook.txt",
                            "media_type": "text/plain",
                            "data_base64": "cnVuYm9vaw=="
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(send.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(send.into_body(), usize::MAX)
        .await
        .unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let attachment_id = sent["attachments"][0]["id"].as_str().unwrap();
    assert_eq!(sent["attachments"][0]["size_bytes"], 7);
    assert!(sent["attachments"][0].get("content").is_none());

    let download = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/private-message-attachments/{attachment_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(download.headers()["content-type"], "text/plain");
    assert_eq!(download.headers()["x-content-type-options"], "nosniff");
    assert_eq!(download.headers()["cache-control"], "private, no-store");
    let bytes = axum::body::to_bytes(download.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"runbook");
}

#[tokio::test]
async fn author_can_edit_and_each_participant_can_toggle_a_reaction() {
    let ctx = test_context();
    let peer = Uuid::new_v4();
    let team = Uuid::new_v4();
    seed_user(&ctx, peer);
    ctx.teams.seed_member(team, me(), Role::Observer);
    ctx.teams.seed_member(team, peer, Role::Observer);
    let message = PrivateMessage::new(me(), peer, "before").unwrap();
    let message_id = message.id;
    ctx.private_messages.seed(message);

    let edit = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/private-messages/{message_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "content": "after" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edit.status(), StatusCode::OK);

    let reaction = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/private-messages/{message_id}/reactions"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "emoji": "✅" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reaction.status(), StatusCode::OK);

    let list = ctx
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/api/private-messages?peer_id={peer}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(list.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["messages"][0]["content"], "after");
    assert!(body["messages"][0]["edited_at"].is_string());
    assert_eq!(body["messages"][0]["reactions"][0]["emoji"], "✅");
    assert_eq!(body["messages"][0]["reactions"][0]["count"], 1);
    assert_eq!(body["messages"][0]["reactions"][0]["reacted"], true);
}

#[tokio::test]
async fn a_participant_cannot_edit_the_other_authors_message() {
    let ctx = test_context();
    let peer = Uuid::new_v4();
    let team = Uuid::new_v4();
    seed_user(&ctx, peer);
    ctx.teams.seed_member(team, me(), Role::Observer);
    ctx.teams.seed_member(team, peer, Role::Observer);
    let message = PrivateMessage::new(peer, me(), "peer authored").unwrap();
    let message_id = message.id;
    ctx.private_messages.seed(message);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/private-messages/{message_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "content": "tampered" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
