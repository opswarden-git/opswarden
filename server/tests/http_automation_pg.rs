mod common;

use std::sync::Arc;

use opswarden_server::adapters::crypto::aes;
use opswarden_server::adapters::crypto::hmac::{hmac_sha256, HmacSha256Verifier};
use opswarden_server::adapters::pg::automation::execution::{
    PgAutomationRunRepo, PgWebhookDeliveryRepo,
};
use opswarden_server::adapters::pg::automation::rule::PgAutomationRuleRepo;
use opswarden_server::adapters::pg::automation::service_connection::{
    PgConnectionCredentialVault, PgServiceConnectionRepo,
};
use opswarden_server::adapters::pg::incident::PgIncidentRepo;
use opswarden_server::adapters::pg::team::PgTeamRepo;
use opswarden_server::adapters::pg::user::PgUserRepo;
use opswarden_server::adapters::webhook::github::GithubParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::app::automation::{
    IngestTeamWebhookCommand, IngestTeamWebhookUseCase, TeamWebhookDependencies,
};
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection,
};
use opswarden_server::domain::team::Team;
use opswarden_server::domain::user::{Email, User};
use opswarden_server::ports::{
    AutomationRuleRepo, AutomationRunRepo, ConnectionCredentialVault, ServiceConnectionRepo,
    TeamRepo, UserRepo,
};
use sqlx::PgPool;
use uuid::Uuid;

const KEY: [u8; aes::KEY_LEN] = [91; aes::KEY_LEN];
const SIGNING_SECRET: &str = "pg-http-signing-secret";
const ENDPOINT: &str = "https://hooks.example.com/pg-secret";
const FAILED_RUN: &[u8] = br#"{
    "repository":{"full_name":"opswarden/pg"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"failure",
        "html_url":"https://github.com/opswarden/pg/actions/runs/94"
    }
}"#;

#[sqlx::test]
async fn postgres_chain_persists_one_http_run_and_deduplicates_the_delivery(pool: PgPool) {
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let user = User::new(
        Email::new(format!("http-pg-{}@test.local", Uuid::new_v4())).unwrap(),
        "hash",
    );
    users.save(&user).await.unwrap();
    let team = Team::new("HTTP automation PG").unwrap();
    teams.save_team(&team).await.unwrap();

    let connections = Arc::new(PgServiceConnectionRepo::new(pool.clone()));
    let credentials = Arc::new(PgConnectionCredentialVault::new(pool.clone(), KEY));
    let rules = Arc::new(PgAutomationRuleRepo::new(pool.clone()));
    let deliveries = Arc::new(PgWebhookDeliveryRepo::new(pool.clone()));
    let runs = Arc::new(PgAutomationRunRepo::new(pool.clone()));
    let notifier = Arc::new(common::DummyNotifier::default());

    let github = ServiceConnection::new(team.id, "github", user.id).unwrap();
    let http = ServiceConnection::new(team.id, "http", user.id).unwrap();
    connections.insert_connection(&github).await.unwrap();
    connections.insert_connection(&http).await.unwrap();
    credentials
        .store_credential(
            github.id,
            CredentialKind::WebhookSigningSecret,
            SIGNING_SECRET,
        )
        .await
        .unwrap();
    credentials
        .store_credential(http.id, CredentialKind::EndpointUrl, ENDPOINT)
        .await
        .unwrap();

    let mut rule = AutomationRule::new(
        team.id,
        "PG CI failed to HTTP",
        github.id,
        "ci_failed",
        serde_json::json!({}),
        "http_notify",
        Some(http.id),
        serde_json::json!({}),
        user.id,
    )
    .unwrap();
    rule.set_enabled(true);
    rules.insert_rule(&rule).await.unwrap();

    let use_case = IngestTeamWebhookUseCase::new(TeamWebhookDependencies {
        connections: connections.clone(),
        credentials: credentials.clone(),
        verifier: Arc::new(HmacSha256Verifier),
        parser: Arc::new(GithubParser),
        deliveries: deliveries.clone(),
        rules: rules.clone(),
        runs: runs.clone(),
        incidents: Arc::new(PgIncidentRepo::new(pool.clone())),
        notifier: notifier.clone(),
        events: Arc::new(WsHub::new()),
    });
    let signature = format!(
        "sha256={}",
        hex::encode(hmac_sha256(SIGNING_SECRET.as_bytes(), FAILED_RUN))
    );

    let first = use_case
        .ingest(command(github.id, signature.clone()))
        .await
        .unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.rules_triggered, 1);
    assert_eq!(notifier.calls().len(), 1);
    let persisted = runs.list_runs_for_team(team.id, 10).await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(persisted[0].incident_id, None);

    let duplicate = use_case
        .ingest(command(github.id, signature))
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(notifier.calls().len(), 1);
    assert_eq!(runs.list_runs_for_team(team.id, 10).await.unwrap().len(), 1);
}

fn command(connection_id: Uuid, signature: String) -> IngestTeamWebhookCommand {
    IngestTeamWebhookCommand {
        connection_id,
        provider_delivery_id: "pg-http-delivery-94".to_string(),
        provider_event: "workflow_run".to_string(),
        signature: Some(signature),
        body: FAILED_RUN.to_vec(),
    }
}
