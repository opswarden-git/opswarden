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

#[tokio::test]
async fn github_oauth_flow_stores_tokens_without_returning_them() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let start = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/api/teams/{team_id}/service-connections/by-service/github/oauth/start?locale=fr"
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::OK);
    let set_cookie = start
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    let cookie = set_cookie.split(';').next().unwrap().to_string();
    let started = json_body(start).await;
    let authorization_url = started["authorization_url"].as_str().unwrap();
    assert!(authorization_url.starts_with("https://github.test/"));
    assert!(authorization_url.contains("code_challenge_method=S256"));
    assert!(!authorization_url.contains(OAUTH_ACCESS));
    assert!(!authorization_url.contains(OAUTH_REFRESH));
    let returned_state = authorization_url
        .split("state=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/service-oauth/github/callback?code=provider-code&state={returned_state}"
                ))
                .header("Cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(callback.status().is_redirection());
    let location = callback
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains(&format!("/fr/teams/{team_id}/automations?view=connections")));
    assert!(!location.contains(OAUTH_ACCESS));
    assert!(!location.contains(OAUTH_REFRESH));

    let connection = ctx
        .service_connections
        .find_connection_by_service(team_id, "github")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthAccessToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_ACCESS)
    );
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_REFRESH)
    );
    let exchanged = ctx.service_oauth.exchanges();
    assert_eq!(exchanged.len(), 1);
    assert_eq!(exchanged[0].0, "provider-code");
    assert!((43..=128).contains(&exchanged[0].1.len()));

    let listed = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    let listed = json_body(listed).await;
    assert_eq!(listed[0]["oauth_configured"], true);
    assert_eq!(listed[0]["oauth_refresh_configured"], true);
    let serialized = listed.to_string();
    for secret in [OAUTH_ACCESS, OAUTH_REFRESH, "provider-code"] {
        assert!(!serialized.contains(secret));
    }
}

#[tokio::test]
async fn github_oauth_refresh_rotates_both_encrypted_credentials() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = ServiceConnection::new(team_id, "github", REQUESTER).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::OAuthAccessToken,
            OAUTH_ACCESS,
        )
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::OAuthRefreshToken,
            OAUTH_REFRESH,
        )
        .await
        .unwrap();

    let refreshed = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/api/teams/{team_id}/service-connections/{}/oauth/refresh",
                connection.id
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(refreshed.status(), StatusCode::OK);
    let response = json_body(refreshed).await;
    assert_eq!(response["oauth_configured"], true);
    assert_eq!(response["oauth_refresh_configured"], true);
    let serialized = response.to_string();
    assert!(!serialized.contains(OAUTH_ACCESS_ROTATED));
    assert!(!serialized.contains(OAUTH_REFRESH_ROTATED));
    assert_eq!(ctx.service_oauth.refreshes(), vec![OAUTH_REFRESH]);
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthAccessToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_ACCESS_ROTATED)
    );
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_REFRESH_ROTATED)
    );
}

#[tokio::test]
async fn github_oauth_callback_rejects_mismatched_state_and_tampered_context() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let started = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/by-service/github/oauth/start"),
            None,
        ))
        .await
        .unwrap();
    let cookie = started
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let authorization_url = json_body(started).await["authorization_url"]
        .as_str()
        .unwrap()
        .to_string();
    let valid_state = authorization_url
        .split("state=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/service-oauth/github/callback?code=provider-code&state=attacker")
                .header("Cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!callback.status().is_success());

    let mut tampered_cookie = cookie.into_bytes();
    let last = tampered_cookie.len() - 1;
    tampered_cookie[last] = if tampered_cookie[last] == b'a' {
        b'b'
    } else {
        b'a'
    };
    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/service-oauth/github/callback?code=provider-code&state={valid_state}"
                ))
                .header("Cookie", String::from_utf8(tampered_cookie).unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!callback.status().is_success());
    assert!(ctx.service_oauth.exchanges().is_empty());
    assert!(ctx.connection_credentials.raw_values().is_empty());
}

#[tokio::test]
async fn manager_configures_and_lists_team_connection_without_secret_material() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_github(&ctx, team_id).await;
    assert_eq!(configured["team_id"], team_id.to_string());
    assert_eq!(configured["service"], "github");
    assert_eq!(configured["secret_configured"], true);
    assert_eq!(configured["token_configured"], true);
    assert_eq!(
        configured["webhook_path"],
        format!("/webhooks/github/{}", configured["id"].as_str().unwrap())
    );
    let serialized = configured.to_string();
    assert!(!serialized.contains(SIGNING_SECRET));
    assert!(!serialized.contains(PERSONAL_TOKEN));
    assert!(!serialized.contains("ciphertext"));
    assert!(!serialized.contains("nonce"));

    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let listed = json_body(response).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let serialized = listed.to_string();
    assert!(!serialized.contains(SIGNING_SECRET));
    assert!(!serialized.contains(PERSONAL_TOKEN));
}

#[tokio::test]
async fn catalog_service_route_configures_known_services_and_rejects_unknown_ones() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/github"),
            Some(serde_json::json!({
                "webhook_signing_secret": SIGNING_SECRET
            })),
        ))
        .await
        .unwrap();
    assert_eq!(configured.status(), StatusCode::OK);
    assert_eq!(json_body(configured).await["service"], "github");

    let gitlab = configure_gitlab(&ctx, team_id).await;
    assert_eq!(gitlab["service"], "gitlab");
    assert_eq!(gitlab["secret_configured"], true);
    assert_eq!(
        gitlab["webhook_path"],
        format!("/webhooks/gitlab/{}", gitlab["id"].as_str().unwrap())
    );
    assert!(!gitlab.to_string().contains(GITLAB_TOKEN));

    let generic = configure_generic(&ctx, team_id).await;
    assert_eq!(generic["service"], "generic");
    assert_eq!(generic["secret_configured"], true);
    assert_eq!(
        generic["webhook_path"],
        format!("/webhooks/generic/{}", generic["id"].as_str().unwrap())
    );
    assert!(!generic.to_string().contains(GENERIC_TOKEN));

    let unknown = ctx
        .app
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/unknown"),
            Some(serde_json::json!({})),
        ))
        .await
        .unwrap();
    assert!(!unknown.status().is_success());
    assert_eq!(
        ctx.service_connections
            .list_connections_for_team(team_id)
            .await
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn manager_configures_and_tests_http_without_exposing_the_endpoint() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_http(&ctx, team_id).await;
    assert_eq!(configured["service"], "http");
    assert_eq!(configured["endpoint_configured"], true);
    assert!(!configured.to_string().contains(HTTP_ENDPOINT));
    let connection_id = configured["id"].as_str().unwrap();
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(
                Uuid::parse_str(connection_id).unwrap(),
                opswarden_server::domain::automation_config::CredentialKind::EndpointUrl,
            )
            .await
            .unwrap()
            .as_deref(),
        Some(HTTP_ENDPOINT)
    );

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::NO_CONTENT);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(ctx.notifier.calls()[0].1, "OpsWarden connection test");
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, Uuid::parse_str(connection_id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.verified_at.is_some());
    assert!(persisted.last_delivery_at.is_none());
    assert_eq!(persisted.last_error_code, None);
}

#[tokio::test]
async fn manager_configures_and_tests_email_without_exposing_the_smtp_password() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_email(&ctx, team_id).await;
    assert_eq!(configured["service"], "email");
    assert!(!configured.to_string().contains(SMTP_PASSWORD));
    let connection_id = Uuid::parse_str(configured["id"].as_str().unwrap()).unwrap();
    for (kind, expected) in [
        (CredentialKind::SmtpHost, "smtp.example.com"),
        (CredentialKind::SmtpPort, "587"),
        (CredentialKind::SmtpUsername, "opswarden"),
        (CredentialKind::SmtpPassword, SMTP_PASSWORD),
        (CredentialKind::FromAddress, "alerts@example.com"),
    ] {
        assert_eq!(
            ctx.connection_credentials
                .reveal_credential(connection_id, kind)
                .await
                .unwrap()
                .as_deref(),
            Some(expected)
        );
    }

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::NO_CONTENT);
    // The probe opens an SMTP session; it must never deliver a message.
    let validated = ctx.email_sender.validated();
    assert_eq!(validated.len(), 1);
    assert_eq!(validated[0].host, "smtp.example.com");
    assert_eq!(validated[0].port, 587);
    assert_eq!(validated[0].from, "alerts@example.com");
    assert!(ctx.email_sender.sent().is_empty());

    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, connection_id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.verified_at.is_some());
    assert_eq!(persisted.last_error_code, None);
}

#[tokio::test]
async fn failed_email_test_records_the_transport_error_code() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let configured = configure_email(&ctx, team_id).await;
    let connection_id = Uuid::parse_str(configured["id"].as_str().unwrap()).unwrap();
    ctx.email_sender
        .fail_with(|| DomainError::EmailTransportError);

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::BAD_GATEWAY);
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, connection_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.last_error_code.as_deref(),
        Some("email_transport_error")
    );
}

#[tokio::test]
async fn creating_an_email_connection_requires_every_credential() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let partial = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/email"),
            Some(json!({"smtp_host": "smtp.example.com", "smtp_port": "587"})),
        ))
        .await
        .unwrap();
    assert_eq!(partial.status(), StatusCode::BAD_REQUEST);
    assert!(ctx
        .service_connections
        .find_connection_by_service(team_id, "email")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn an_email_connection_rejects_a_malformed_port_or_sender() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    for payload in [
        json!({
            "smtp_host": "smtp.example.com",
            "smtp_port": "not-a-port",
            "smtp_username": "opswarden",
            "smtp_password": SMTP_PASSWORD,
            "from_address": "alerts@example.com"
        }),
        json!({
            "smtp_host": "smtp.example.com",
            "smtp_port": "587",
            "smtp_username": "opswarden",
            "smtp_password": SMTP_PASSWORD,
            "from_address": "not-an-address"
        }),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/email"),
                Some(payload),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn a_responder_cannot_configure_an_email_connection() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Responder);

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
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn failed_http_test_records_only_a_safe_error_code() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let configured = configure_http(&ctx, team_id).await;
    let connection_id = configured["id"].as_str().unwrap();
    ctx.notifier.fail_requests();

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(json_body(tested).await["code"], "reaction_http_5xx");
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, Uuid::parse_str(connection_id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.last_error_code.as_deref(),
        Some("reaction_http_5xx")
    );
    assert!(!format!("{persisted:?}").contains(HTTP_ENDPOINT));
}

#[tokio::test]
async fn only_manager_can_read_connections_or_runs() {
    for role in [Role::Responder, Role::Observer] {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        ctx.teams.seed_member(team_id, REQUESTER, role);

        for suffix in ["service-connections", "automation-rules", "automation-runs"] {
            let response = ctx
                .app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("/api/teams/{team_id}/{suffix}"),
                    None,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert_eq!(json_body(response).await["code"], "not_manager");
        }

        let configure = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/github"),
                Some(json!({"webhook_signing_secret": SIGNING_SECRET})),
            ))
            .await
            .unwrap();
        assert_eq!(configure.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure).await["code"], "not_manager");

        let configure_generic = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/generic"),
                Some(json!({"webhook_signing_secret": GENERIC_TOKEN})),
            ))
            .await
            .unwrap();
        assert_eq!(configure_generic.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure_generic).await["code"], "not_manager");

        let configure_http = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/http"),
                Some(json!({"endpoint_url": HTTP_ENDPOINT})),
            ))
            .await
            .unwrap();
        assert_eq!(configure_http.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure_http).await["code"], "not_manager");

        let test_http = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!(
                    "/api/teams/{team_id}/service-connections/{}/test",
                    Uuid::new_v4()
                ),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(test_http.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(test_http).await["code"], "not_manager");
        assert!(ctx.connection_credentials.raw_values().is_empty());
    }

    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let response = ctx
        .app
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(response).await["code"], "forbidden");
}

#[tokio::test]
async fn manager_of_team_a_cannot_read_or_delete_team_b_connection() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    ctx.teams.seed_member(team_a, REQUESTER, Role::Manager);
    let owner_b = Uuid::new_v4();
    let connection_b = ServiceConnection::new(team_b, "github", owner_b).unwrap();
    ctx.service_connections
        .insert_connection(&connection_b)
        .await
        .unwrap();

    let read = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_b}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::FORBIDDEN);

    let delete = ctx
        .app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!(
                "/api/teams/{team_b}/service-connections/{}",
                connection_b.id
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::FORBIDDEN);
    assert!(ctx
        .service_connections
        .find_connection_for_team(team_b, connection_b.id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn manager_creates_updates_lists_and_deletes_a_disabled_by_default_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_github(&ctx, team_id).await;
    let connection_id = connection["id"].as_str().unwrap();

    let create = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "CI failed -> incident",
                "trigger_connection_id": connection_id,
                "trigger_kind": "ci_failed",
                "trigger_config": {"repository": "opswarden/app"},
                "reaction_kind": "create_incident",
                "reaction_config": {"severity": "high"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = json_body(create).await;
    assert_eq!(created["enabled"], false);
    let rule_id = created["id"].as_str().unwrap();

    let update = ctx
        .app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/teams/{team_id}/automation-rules/{rule_id}"),
            Some(json!({"name": "Production CI failed", "enabled": true})),
        ))
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let updated = json_body(update).await;
    assert_eq!(updated["name"], "Production CI failed");
    assert_eq!(updated["enabled"], true);

    let list = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/automation-rules"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(list).await.as_array().unwrap().len(), 1);

    let delete = ctx
        .app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/teams/{team_id}/automation-rules/{rule_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    assert!(ctx
        .automation_rules
        .list_rules_for_team(team_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn manager_can_create_every_catalogued_github_action_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_github(&ctx, team_id).await;
    let connection_id = connection["id"].as_str().unwrap();
    let cases = [
        (
            "ci_succeeded",
            json!({"repository": "opswarden/app", "branch": "main"}),
            "{{workflow}} succeeded",
        ),
        (
            "tag_pushed",
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            "Tag {{tag}} pushed by {{actor}}",
        ),
        (
            "pr_merged",
            json!({"repository": "opswarden/app", "branch": "main", "source_branch": "feature/opswarden"}),
            "PR #{{pull_request_number}} {{pull_request_title}}",
        ),
    ];

    for (trigger_kind, trigger_config, title) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("GitHub {trigger_kind}"),
                    "trigger_connection_id": connection_id,
                    "trigger_kind": trigger_kind,
                    "trigger_config": trigger_config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high", "title": title}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{trigger_kind}");
        let rule = json_body(response).await;
        assert_eq!(rule["trigger_kind"], trigger_kind);
        assert_eq!(rule["enabled"], false);
    }
}

#[tokio::test]
async fn manager_can_create_every_catalogued_gitlab_action_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_gitlab(&ctx, team_id).await;
    let cases = [
        (
            "ci_failed",
            json!({"repository": "opswarden/app", "branch": "main"}),
        ),
        (
            "ci_succeeded",
            json!({"repository": "opswarden/app", "branch": "main"}),
        ),
        (
            "tag_pushed",
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
        ),
    ];
    for (trigger_kind, trigger_config) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("GitLab {trigger_kind}"),
                    "trigger_connection_id": connection["id"],
                    "trigger_kind": trigger_kind,
                    "trigger_config": trigger_config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high"}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{trigger_kind}");
        assert_eq!(json_body(response).await["trigger_kind"], trigger_kind);
    }
}

#[tokio::test]
async fn manager_can_create_bounded_timer_rules_and_invalid_schedule_is_rejected() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let timer = ServiceConnection::new_internal(team_id, "timer").unwrap();
    ctx.service_connections
        .insert_connection(&timer)
        .await
        .unwrap();

    for (name, kind, config) in [
        (
            "Daily handover",
            "daily_at",
            json!({"time": "09:30", "timezone": "Europe/Paris"}),
        ),
        (
            "Frequent check",
            "every_minutes",
            json!({"minutes": "15", "timezone": "UTC"}),
        ),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": name,
                    "trigger_connection_id": timer.id,
                    "trigger_kind": kind,
                    "trigger_config": config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high"}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{kind}");
    }

    let invalid = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Invalid timer",
                "trigger_connection_id": timer.id,
                "trigger_kind": "every_minutes",
                "trigger_config": {"minutes": "4", "timezone": "UTC"},
                "reaction_kind": "create_incident",
                "reaction_config": {"severity": "high"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(invalid).await["code"], "invalid_timer_schedule");
}

#[tokio::test]
async fn internal_automation_connections_cannot_be_deleted() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    for service in ["opswarden", "timer"] {
        let connection = ServiceConnection::new_internal(team_id, service).unwrap();
        ctx.service_connections
            .insert_connection(&connection)
            .await
            .unwrap();
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "DELETE",
                &format!("/api/teams/{team_id}/service-connections/{}", connection.id),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{service}");
        assert!(ctx
            .service_connections
            .find_connection_by_id(connection.id)
            .await
            .unwrap()
            .is_some());
    }
}

#[tokio::test]
async fn manager_can_create_a_filtered_generic_event_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_generic(&ctx, team_id).await;
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Generic deployment failed",
                "trigger_connection_id": connection["id"],
                "trigger_kind": "generic_event",
                "trigger_config": {
                    "event_type": "deployment_failed",
                    "source": "jury",
                    "severity": "critical"
                },
                "reaction_kind": "create_incident",
                "reaction_config": {
                    "severity": "critical",
                    "title": "{{source}}: {{title}} ({{external_id}})"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_body(response).await["enabled"], false);
}

#[tokio::test]
async fn manager_can_create_every_catalogued_native_opswarden_reaction_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let github = configure_github(&ctx, team_id).await;
    let release_id = Uuid::new_v4();
    let incident_id = Uuid::new_v4();
    let cases = [
        (
            "validate_release_step",
            json!({"release_id": release_id, "step": "build"}),
        ),
        (
            "block_release",
            json!({
                "release_id": release_id,
                "severity": "critical",
                "title": "{{workflow}} blocks the release"
            }),
        ),
        ("escalate_incident", json!({"incident_id": incident_id})),
    ];

    for (reaction_kind, reaction_config) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("CI failed to {reaction_kind}"),
                    "trigger_connection_id": github["id"],
                    "trigger_kind": "ci_failed",
                    "trigger_config": {},
                    "reaction_kind": reaction_kind,
                    "reaction_config": reaction_config
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{reaction_kind}");
        let rule = json_body(response).await;
        assert_eq!(rule["reaction_kind"], reaction_kind);
        assert_eq!(rule["enabled"], false);
    }
}

#[tokio::test]
async fn http_rule_requires_its_own_team_connection_and_a_catalog_bounded_payload() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let github = configure_github(&ctx, team_id).await;
    let http = configure_http(&ctx, team_id).await;

    let valid = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "CI failed -> HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {
                    "message": "{{workflow}} failed on {{repository}}"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(valid.status(), StatusCode::CREATED);
    assert_eq!(json_body(valid).await["enabled"], false);

    let configurable_payload = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Unsafe customizable HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {"headers": {"Authorization": "secret"}}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(configurable_payload.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(configurable_payload).await["code"],
        "invalid_automation_rule"
    );

    let unknown_variable = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Secret template",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {"message": "{{oauth_access_token}}"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(unknown_variable.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(unknown_variable).await["code"],
        "invalid_automation_rule"
    );

    let team_b = Uuid::new_v4();
    let foreign_http = ServiceConnection::new(team_b, "http", Uuid::new_v4()).unwrap();
    ctx.service_connections
        .insert_connection(&foreign_http)
        .await
        .unwrap();
    let cross_team = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Cross-Team HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": foreign_http.id,
                "reaction_config": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(cross_team.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json_body(cross_team).await["code"],
        "service_connection_not_found"
    );
}

#[tokio::test]
async fn cross_team_trigger_and_secret_shaped_rule_config_are_rejected() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    ctx.teams.seed_member(team_a, REQUESTER, Role::Manager);
    let connection_b = ServiceConnection::new(team_b, "github", Uuid::new_v4()).unwrap();
    ctx.service_connections
        .insert_connection(&connection_b)
        .await
        .unwrap();

    let base = json!({
        "name": "bad rule",
        "trigger_connection_id": connection_b.id,
        "trigger_kind": "ci_failed",
        "trigger_config": {},
        "reaction_kind": "create_incident",
        "reaction_config": {}
    });
    let cross_team = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_a}/automation-rules"),
            Some(base),
        ))
        .await
        .unwrap();
    assert_eq!(cross_team.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json_body(cross_team).await["code"],
        "service_connection_not_found"
    );

    let own_connection = configure_github(&ctx, team_a).await;
    let leaky = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_a}/automation-rules"),
            Some(json!({
                "name": "leaky rule",
                "trigger_connection_id": own_connection["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {"access_token": "must-not-be-persisted"},
                "reaction_kind": "create_incident",
                "reaction_config": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(leaky.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(leaky).await["code"], "invalid_automation_rule");
}

#[tokio::test]
async fn team_automation_routes_require_authentication() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/api/teams/{team_id}/automation-rules"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn release_creation_triggers_a_durable_native_opswarden_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let opswarden = ServiceConnection::new_internal(team_id, "opswarden").unwrap();
    ctx.service_connections
        .insert_connection(&opswarden)
        .await
        .unwrap();

    let created = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Release created -> Incident",
                "trigger_connection_id": opswarden.id,
                "trigger_kind": "release_created",
                "trigger_config": {},
                "reaction_kind": "create_incident",
                "reaction_config": {
                    "severity": "high",
                    "title": "Release {{release_title}} requires coordination"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let rule = json_body(created).await;
    let enabled = ctx
        .app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!(
                "/api/teams/{team_id}/automation-rules/{}",
                rule["id"].as_str().unwrap()
            ),
            Some(json!({"enabled": true})),
        ))
        .await
        .unwrap();
    assert_eq!(enabled.status(), StatusCode::OK);

    let release = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            "/api/releases",
            Some(json!({
                "team_id": team_id,
                "title": "v2.0.0",
                "steps": ["build", "production"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(release.status(), StatusCode::CREATED);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].title, "Release v2.0.0 requires coordination");
    assert_eq!(incidents[0].severity.to_string(), "high");
    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].provider_event, "release_created");
    assert_eq!(deliveries[0].status.to_string(), "processed");
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status.to_string(), "succeeded");
    assert_eq!(runs[0].incident_id, Some(incidents[0].id));
}
