mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::test_context;
use opswarden_server::domain::automation_config::ServiceConnection;
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
async fn http_rule_requires_its_own_team_connection_and_a_fixed_payload_contract() {
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
                "reaction_config": {}
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
