// --- server/tests/middleware.rs ---

mod common;
use common::test_app;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn api_me_returns_unauthorized_without_token() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_me_returns_unauthorized_with_invalid_token() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/me")
                .header("Authorization", "Bearer invalid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_me_returns_ok_with_valid_token() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/me")
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
    assert_eq!(
        json["id"].as_str().unwrap(),
        "00000000-0000-0000-0000-000000000000"
    );
}
