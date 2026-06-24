mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use tower::ServiceExt;

#[tokio::test]
async fn giphy_search_returns_normalized_results_for_an_authed_user() {
    let ctx = test_context();

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/giphy/search?q=deploy&limit=5")
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
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["url"],
        "https://media.giphy.com/media/demo/giphy.gif"
    );
    assert!(arr[0]["preview_url"]
        .as_str()
        .unwrap()
        .starts_with("https://media.giphy.com/"));
}

#[tokio::test]
async fn giphy_search_rejects_a_blank_query() {
    let ctx = test_context();

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/giphy/search?q=")
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
