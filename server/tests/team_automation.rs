mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::test_context;
use opswarden_server::domain::automation_config::{CredentialKind, ServiceConnection};
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::team::Role;
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const AUTH: &str = "Bearer mock_jwt_token";
const REQUESTER: Uuid = Uuid::nil();
const SIGNING_SECRET: &str = "github-signing-secret-never-returned";
const GITLAB_TOKEN: &str = "gitlab-webhook-token-never-returned";
const GENERIC_TOKEN: &str = "generic-webhook-token-never-returned";
const PERSONAL_TOKEN: &str = "github_pat_never_returned";
const HTTP_ENDPOINT: &str = "https://hooks.example.com/services/secret-path";
const SMTP_PASSWORD: &str = "smtp-password-never-returned";
const OAUTH_ACCESS: &str = "github_oauth_access_never_returned";
const OAUTH_REFRESH: &str = "github_oauth_refresh_never_returned";
const OAUTH_ACCESS_ROTATED: &str = "github_oauth_access_rotated";
const OAUTH_REFRESH_ROTATED: &str = "github_oauth_refresh_rotated";

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", AUTH);
    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

async fn configure_github(ctx: &common::TestContext, team_id: Uuid) -> Value {
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/github"),
            Some(json!({
                "webhook_signing_secret": SIGNING_SECRET,
                "personal_token": PERSONAL_TOKEN
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn configure_http(ctx: &common::TestContext, team_id: Uuid) -> Value {
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/http"),
            Some(json!({"endpoint_url": HTTP_ENDPOINT})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

fn email_payload() -> Value {
    json!({
        "smtp_host": "smtp.example.com",
        "smtp_port": "587",
        "smtp_username": "opswarden",
        "smtp_password": SMTP_PASSWORD,
        "from_address": "alerts@example.com"
    })
}

async fn configure_email(ctx: &common::TestContext, team_id: Uuid) -> Value {
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/email"),
            Some(email_payload()),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn configure_gitlab(ctx: &common::TestContext, team_id: Uuid) -> Value {
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/gitlab"),
            Some(json!({"webhook_signing_secret": GITLAB_TOKEN})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn configure_generic(ctx: &common::TestContext, team_id: Uuid) -> Value {
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/generic"),
            Some(json!({"webhook_signing_secret": GENERIC_TOKEN})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

include!("team_automation/oauth.rs");
include!("team_automation/connections.rs");
include!("team_automation/rule_catalog.rs");
include!("team_automation/rule_security.rs");
include!("team_automation/native.rs");
