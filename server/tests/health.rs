// --- server/tests/health.rs ---

mod common;
use common::test_app;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
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
async fn about_exposes_the_request_client_and_complete_server_contract() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let peer: SocketAddr = "203.0.113.42:43123".parse().unwrap();
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/about.json")
                .header("X-Forwarded-For", "192.0.2.99")
                .extension(ConnectInfo(peer))
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
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(json["client"]["host"], "203.0.113.42");
    assert_eq!(
        json["client"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["host"]
    );
    let current_time = json["server"]["current_time"].as_u64().unwrap();
    assert!((before..=after).contains(&current_time));
    let token = json["server"]["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        json["server"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["current_time", "services", "token"]
    );

    let services = json["server"]["services"].as_array().unwrap();
    assert_eq!(services.len(), 3);
    assert!(services.iter().any(|service| {
        service["name"] == "github"
            && service["actions"][0]["name"] == "ci_failed"
            && service["actions"][0]["connection_service"] == "github"
            && service["actions"][0]["fields"][0]["name"] == "repository"
            && service["connection"]["fields"][0]["name"] == "webhook_signing_secret"
            && service["connection"]["oauth"]["label"] == "Authorize with GitHub"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "vigil" && service["reactions"][0]["name"] == "vigil_create_incident"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "http"
            && service["reactions"][0]["name"] == "http_notify"
            && service["reactions"][0]["fields"][0]["name"] == "message"
            && service["reactions"][0]["fields"][0]["default_value"]
                == "{{workflow}} failed on {{repository}}"
            && service["connection"]["fields"][0]["name"] == "endpoint_url"
            && service["connection"]["testable"] == true
    }));
}

#[tokio::test]
async fn about_localizes_the_server_owned_catalog_in_french() {
    let peer: SocketAddr = "203.0.113.42:43123".parse().unwrap();
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/about.json?locale=fr")
                .extension(ConnectInfo(peer))
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
    let services = json["server"]["services"].as_array().unwrap();
    let github = services
        .iter()
        .find(|service| service["name"] == "github")
        .unwrap();
    assert_eq!(github["actions"][0]["label"], "Échec d’un workflow CI");
    assert_eq!(
        github["connection"]["fields"][0]["label"],
        "Secret de signature du webhook"
    );
    assert_eq!(
        github["connection"]["oauth"]["label"],
        "Autoriser avec GitHub"
    );
    let vigil = services
        .iter()
        .find(|service| service["name"] == "vigil")
        .unwrap();
    assert_eq!(
        vigil["reactions"][0]["fields"][0]["options"][3],
        serde_json::json!({"value": "critical", "label": "Critique"})
    );
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
