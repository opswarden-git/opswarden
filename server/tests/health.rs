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
    let service_names: Vec<_> = services
        .iter()
        .map(|service| service["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        service_names,
        [
            "github",
            "gitlab",
            "generic",
            "alertmanager",
            "opswarden",
            "timer",
            "http",
            "email"
        ]
    );
    assert!(services.iter().any(|service| {
        service["name"] == "github"
            && service["actions"][0]["name"] == "ci_failed"
            && service["actions"][1]["name"] == "ci_succeeded"
            && service["actions"][2]["name"] == "tag_pushed"
            && service["actions"][3]["name"] == "pr_merged"
            && service["actions"][0]["connection_service"] == "github"
            && service["actions"][0]["fields"][0]["name"] == "repository"
            && service["connection"]["fields"][0]["name"] == "webhook_signing_secret"
            && service["connection"]["oauth"]["label"] == "Authorize with GitHub"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "gitlab"
            && service["actions"][0]["name"] == "ci_failed"
            && service["actions"][1]["name"] == "ci_succeeded"
            && service["actions"][2]["name"] == "tag_pushed"
            && service["connection"]["fields"][0]["name"] == "webhook_signing_secret"
            && service["connection"]["oauth"].is_null()
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "generic"
            && service["actions"][0]["name"] == "generic_event"
            && service["actions"][0]["fields"][0]["name"] == "event_type"
            && service["connection"]["fields"][0]["name"] == "webhook_signing_secret"
            && service["connection"]["oauth"].is_null()
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "alertmanager"
            && service["actions"][0]["name"] == "alert_firing"
            && service["actions"][0]["connection_service"] == "alertmanager"
            && service["actions"][0]["fields"][0]["name"] == "severity"
            && service["connection"]["fields"][0]["name"] == "webhook_signing_secret"
            && service["connection"]["fields"][0]["label"] == "Bearer token"
            && service["connection"]["testable"] == false
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "opswarden"
            && service["actions"][0]["name"] == "release_created"
            && service["actions"][0]["connection_service"] == "opswarden"
            && service["reactions"][0]["name"] == "create_incident"
            && service["reactions"][1]["name"] == "validate_release_step"
            && service["reactions"][2]["name"] == "block_release"
            && service["reactions"][3]["name"] == "escalate_incident"
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "timer"
            && service["actions"][0]["name"] == "daily_at"
            && service["actions"][0]["fields"][0]["input_type"] == "time"
            && service["actions"][1]["name"] == "every_minutes"
            && service["actions"][1]["fields"][0]["input_type"] == "number"
            && service["connection"].is_null()
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "http"
            && service["reactions"][0]["name"] == "http_notify"
            && service["reactions"][0]["fields"][0]["name"] == "message"
            && service["reactions"][0]["fields"][0]["default_value"]
                == "Automation event on {{repository}}"
            && service["connection"]["fields"][0]["name"] == "endpoint_url"
            && service["connection"]["testable"] == true
    }));
    assert!(services.iter().any(|service| {
        service["name"] == "email"
            && service["reactions"][0]["name"] == "email_notify"
            && service["reactions"][0]["fields"][0]["name"] == "to"
            && service["connection"]["fields"][0]["name"] == "smtp_host"
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
    assert_eq!(github["actions"][1]["label"], "Succès d’un workflow CI");
    assert_eq!(github["actions"][2]["label"], "Nouveau tag poussé");
    assert_eq!(github["actions"][3]["label"], "Pull request fusionnée");
    assert_eq!(
        github["connection"]["fields"][0]["label"],
        "Secret de signature du webhook"
    );
    let gitlab = services
        .iter()
        .find(|service| service["name"] == "gitlab")
        .unwrap();
    assert_eq!(gitlab["actions"][0]["label"], "Échec d’une pipeline CI");
    assert_eq!(gitlab["actions"][1]["label"], "Succès d’une pipeline CI");
    assert_eq!(gitlab["actions"][2]["label"], "Nouveau tag poussé");
    let generic = services
        .iter()
        .find(|service| service["name"] == "generic")
        .unwrap();
    let alertmanager = services
        .iter()
        .find(|service| service["name"] == "alertmanager")
        .unwrap();
    assert_eq!(
        alertmanager["actions"][0]["label"],
        "Groupe d’alertes actif"
    );
    assert_eq!(
        alertmanager["connection"]["fields"][0]["label"],
        "Jeton Bearer"
    );
    assert_eq!(generic["actions"][0]["label"], "Événement JSON générique");
    assert_eq!(
        generic["connection"]["fields"][0]["label"],
        "Jeton partagé du webhook"
    );
    assert_eq!(
        github["connection"]["oauth"]["label"],
        "Autoriser avec GitHub"
    );
    let opswarden = services
        .iter()
        .find(|service| service["name"] == "opswarden")
        .unwrap();
    assert_eq!(opswarden["actions"][0]["label"], "Release créée");
    assert_eq!(
        opswarden["reactions"][1]["label"],
        "Valider une étape de Release"
    );
    assert_eq!(opswarden["reactions"][2]["label"], "Bloquer une Release");
    assert_eq!(opswarden["reactions"][3]["label"], "Escalader un Incident");
    let timer = services
        .iter()
        .find(|service| service["name"] == "timer")
        .unwrap();
    assert_eq!(
        timer["actions"][0]["label"],
        "Tous les jours à une heure locale"
    );
    assert_eq!(timer["actions"][1]["label"], "Toutes les N minutes");
    assert_eq!(
        opswarden["reactions"][0]["fields"][0]["options"][3],
        serde_json::json!({"value": "critical", "label": "Critique"})
    );
    let email = services
        .iter()
        .find(|service| service["name"] == "email")
        .unwrap();
    assert_eq!(email["connection"]["fields"][0]["label"], "Hôte SMTP");
    assert_eq!(email["reactions"][0]["label"], "Envoyer un e-mail");
    assert_eq!(
        email["reactions"][0]["fields"][0]["label"],
        "Destinataire (À)"
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
