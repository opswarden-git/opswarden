mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::ports::SecretVault;
use tower::ServiceExt;

const AUTH: &str = "Bearer mock_jwt_token";
const SECRET: &str = "super-secret-hmac-key";

async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn configure_github_then_status_reports_connected_without_leaking_secret() {
    let ctx = test_context();

    // Initially disconnected.
    let res = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/service-connections")
                .header("Authorization", AUTH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body_string(res).await).unwrap();
    assert_eq!(body["github"]["connected"], false);

    // Configure GitHub.
    let res = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/service-connections/github")
                .header("Authorization", AUTH)
                .header("Content-Type", "application/json")
                .body(Body::from(format!(r#"{{"webhook_secret":"{SECRET}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // The response must never echo the raw secret.
    assert!(!body_string(res).await.contains(SECRET));

    // It is actually stored in the vault (here the in-memory test vault).
    assert_eq!(
        ctx.vault.reveal("github").await.unwrap().as_deref(),
        Some(SECRET)
    );

    // Status now reports connected, still without leaking the secret.
    let res = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/service-connections")
                .header("Authorization", AUTH)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status_body = body_string(res).await;
    assert!(!status_body.contains(SECRET));
    let body: serde_json::Value = serde_json::from_str(&status_body).unwrap();
    assert_eq!(body["github"]["connected"], true);
}

#[tokio::test]
async fn configure_github_requires_authentication() {
    let ctx = test_context();
    let res = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/service-connections/github")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"webhook_secret":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn empty_secret_is_rejected_and_stores_nothing() {
    let ctx = test_context();
    let res = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/service-connections/github")
                .header("Authorization", AUTH)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"webhook_secret":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert!(ctx.vault.reveal("github").await.unwrap().is_none());
}
