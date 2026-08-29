// server/tests/auth.rs

mod common;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::user::Locale;
use opswarden_server::ports::UserRepo;
use serde_json::json;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tower::ServiceExt;

/// The auth routes carry a per-caller budget, so every request must arrive with
/// the peer address the middleware keys on. Production supplies it through
/// `into_make_service_with_connect_info`; `oneshot` needs it injected.
fn client_addr() -> ConnectInfo<SocketAddr> {
    ConnectInfo("203.0.113.7:54321".parse().unwrap())
}

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
                .extension(client_addr())
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
                .extension(client_addr())
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
                .extension(client_addr())
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
                .extension(client_addr())
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
                .extension(client_addr())
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
                .extension(client_addr())
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
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events.register(uuid::Uuid::nil(), HashSet::new(), tx);

    let logout = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    assert!(rx.recv().await.is_none());

    let me = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/me")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_exposes_and_updates_the_persisted_supported_locale() {
    let ctx = test_context();
    let update = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/me/locale")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"locale": "fr"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let body = axum::body::to_bytes(update.into_body(), usize::MAX)
        .await
        .unwrap();
    let profile: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(profile["locale"], "fr");
    assert!(profile["created_at"].is_string());

    let me = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/me")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(me.into_body(), usize::MAX)
        .await
        .unwrap();
    let profile: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(profile["locale"], "fr");
    assert!(profile["created_at"].is_string());
    assert_eq!(
        ctx.users
            .find_by_id(uuid::Uuid::nil())
            .await
            .unwrap()
            .unwrap()
            .locale,
        Locale::Fr
    );
}

#[tokio::test]
async fn locale_update_rejects_values_outside_english_and_french() {
    let response = test_context()
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/me/locale")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"locale": "de"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["code"], "invalid_locale");
}

#[tokio::test]
async fn delete_me_removes_the_authenticated_account() {
    let ctx = test_context();
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events.register(uuid::Uuid::nil(), HashSet::new(), tx);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/me")
                .extension(client_addr())
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn google_start_redirects_and_sets_state_cookie() {
    let response = test_context()
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/google/start")
                .extension(client_addr())
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

include!("auth/rate_limit.rs");
