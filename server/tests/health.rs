use axum::body::Body;
use axum::http::{Request, StatusCode};
use opswarden_server::{build_app, config::Config};
use tower::ServiceExt; // brings `oneshot`

fn test_app() -> axum::Router {
    build_app(Config {
        kickoff_token_secret: "test-secret".to_string(),
    })
}

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
}
