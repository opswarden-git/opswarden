// --- server/tests/health.rs ---

mod common;
use common::test_app;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn about_exposes_a_64_char_token() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/about.json")
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
    let token = json["server"]["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);

    let services = json["server"]["services"].as_array().unwrap();
    assert_eq!(services.len(), 3);
    assert!(services.iter().any(|service| {
        service["name"] == "github"
            && service["actions"][0]["name"] == "ci_failed"
            && service["actions"][0]["connection_service"] == "github"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "vigil" && service["reactions"][0]["name"] == "vigil_create_incident"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "http" && service["reactions"][0]["name"] == "http_notify"
    }));
}

#[tokio::test]
async fn legacy_global_automation_routes_are_gone() {
    for (method, uri) in [
        ("GET", "/api/service-connections"),
        ("PUT", "/api/service-connections/github"),
        ("POST", "/webhooks/github"),
    ] {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {uri}");
    }
}
