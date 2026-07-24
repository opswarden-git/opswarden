mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::test_context;
use opswarden_server::domain::automation_config::{CredentialKind, ServiceConnection};
use opswarden_server::domain::team::Role;
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const AUTH: &str = "Bearer mock_jwt_token";
const REQUESTER: Uuid = Uuid::nil();
const SIGNING_SECRET: &str = "github-signing-secret-never-returned";
const PERSONAL_TOKEN: &str = "github_pat_never_returned";
const HTTP_ENDPOINT: &str = "https://hooks.example.com/services/secret-path";
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
        1
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
                "reaction_kind": "vigil_create_incident",
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
        "reaction_kind": "vigil_create_incident",
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
                "reaction_kind": "vigil_create_incident",
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
