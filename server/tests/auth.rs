// server/tests/auth.rs

mod common;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use common::test_context;
use tower::ServiceExt;

#[tokio::test]
async fn signup_returns_created_for_new_user() {
    let payload = serde_json::json!({
        "email": "new@test.com",
        "password": "password123"
    });

    let response = test_context()
        .app
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

    let response = test_context()
        .app
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

    let response = test_context()
        .app
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

    let response = test_context()
        .app
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

    let response = test_context()
        .app
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

    let response = test_context()
        .app
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
async fn logout_revokes_the_bearer_token() {
    let ctx = test_context();

    let logout = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let me = ctx
        .app
        .clone()
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

    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_me_removes_the_authenticated_account() {
    let response = test_context()
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/me")
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn google_start_redirects_and_sets_state_cookie() {
    let response = test_context()
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/google/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with("https://accounts.google.test/auth?state="));
    assert!(response.headers().get(header::SET_COOKIE).is_some());
}

#[tokio::test]
async fn google_callback_exchanges_code_and_redirects_with_token() {
    let state = "oauth-test-state";
    let response = test_context()
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/auth/google/callback?code=ok&state={state}"))
                .header(header::COOKIE, format!("opswarden_oauth_state={state}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(
        location,
        "http://localhost:4242/en/login#oauth_token=mock_jwt_token"
    );
}
