// server/tests/auth.rs

mod common;
use common::test_app;
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;

#[tokio::test]
async fn signup_returns_created_for_new_user() {
    let payload = serde_json::json!({
        "email": "new@test.com",
        "password": "password123"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-up")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn signup_returns_conflict_for_existing_user() {
    let payload = serde_json::json!({
        "email": "existing@test.com",
        "password": "password123"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-up")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn signup_returns_bad_request_for_invalid_email() {
    let payload = serde_json::json!({
        "email": "invalid-email",
        "password": "password123"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-up")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn signin_returns_ok_with_token_for_valid_credentials() {
    let payload = serde_json::json!({
        "email": "existing@test.com",
        "password": "correct_password"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.get("token").is_some());
    assert_eq!(json["token"].as_str().unwrap(), "mock_jwt_token");
}

#[tokio::test]
async fn signin_returns_unauthorized_for_invalid_password() {
    let payload = serde_json::json!({
        "email": "existing@test.com",
        "password": "wrong_password"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn signin_returns_unauthorized_for_unknown_user() {
    let payload = serde_json::json!({
        "email": "unknown@test.com",
        "password": "correct_password"
    });

    let response = test_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
