mod common;

use axum::{body::Body, http::Request, http::StatusCode, response::Response};
use common::test_context;
use opswarden_server::domain::automation_config::CredentialKind;
use opswarden_server::domain::team::Role;
use opswarden_server::ports::{ConnectionCredentialVault, ServiceConnectionRepo};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const REQUESTER: Uuid = Uuid::nil();
const TOKEN: &str = "alertmanager-token-never-returned";

fn configure_request(team_id: Uuid, payload: Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/teams/{team_id}/service-connections/by-service/alertmanager"
        ))
        .header("Authorization", "Bearer mock_jwt_token")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn manager_configures_alertmanager_without_exposing_the_bearer_token() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let response = ctx
        .app
        .clone()
        .oneshot(configure_request(
            team_id,
            json!({"webhook_signing_secret": TOKEN}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["service"], "alertmanager");
    assert_eq!(body["secret_configured"], true);
    assert_eq!(
        body["webhook_path"],
        format!("/webhooks/alertmanager/{}", body["id"].as_str().unwrap())
    );
    assert!(!body.to_string().contains(TOKEN));

    let connection = ctx
        .service_connections
        .find_connection_by_service(team_id, "alertmanager")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::WebhookSigningSecret)
            .await
            .unwrap()
            .as_deref(),
        Some(TOKEN)
    );
}

#[tokio::test]
async fn first_configuration_requires_a_non_empty_token() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    for payload in [json!({}), json!({"webhook_signing_secret": "  "})] {
        let response = ctx
            .app
            .clone()
            .oneshot(configure_request(team_id, payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["code"], "invalid_service_secret");
    }
    assert!(ctx
        .service_connections
        .find_connection_by_service(team_id, "alertmanager")
        .await
        .unwrap()
        .is_none());
}
