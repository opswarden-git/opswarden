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
use opswarden_server::adapters::pg::release::PgReleaseRepo;
use opswarden_server::adapters::pg::team::PgTeamRepo;
use opswarden_server::adapters::pg::user::PgUserRepo;
use opswarden_server::adapters::webhook::generic::GenericParser;
use opswarden_server::adapters::webhook::github::GithubParser;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::app::automation::{
    release_created_event, DispatchInternalAutomationCommand, DispatchInternalAutomationUseCase,
    IngestTeamWebhookCommand, IngestTeamWebhookUseCase, InternalAutomationDependencies,
    TeamWebhookDependencies,
};
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection,
};
use opswarden_server::domain::release::Release;
use opswarden_server::domain::team::{Role, Team};
use opswarden_server::domain::user::{Email, User};
use opswarden_server::ports::{
    AutomationRuleRepo, AutomationRunRepo, ConnectionCredentialVault, IncidentRepo, ReleaseRepo,
    ServiceConnectionRepo, TeamRepo, UserRepo,
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
const GENERIC_BODY: &[u8] = br#"{
    "source":"pg-monitor",
    "title":"Database unavailable",
    "severity":"critical",
    "external_id":"pg-alert-42"
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
        releases: Arc::new(opswarden_server::adapters::pg::release::PgReleaseRepo::new(
            pool.clone(),
        )),
        notifier: notifier.clone(),
        events: Arc::new(WsHub::new()),
        email_sender: Arc::new(opswarden_server::adapters::email::SmtpEmailSender::new()),
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

#[sqlx::test]
async fn postgres_internal_release_event_creates_one_incident_and_one_durable_run(pool: PgPool) {
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let manager = User::new(
        Email::new(format!("opswarden-pg-{}@test.local", Uuid::new_v4())).unwrap(),
        "hash",
    );
    users.save(&manager).await.unwrap();
    let team = Team::new("Native OpsWarden PG").unwrap();
    teams.save_team(&team).await.unwrap();
    teams
        .add_member(team.id, manager.id, Role::Manager)
        .await
        .unwrap();

    let connections = Arc::new(PgServiceConnectionRepo::new(pool.clone()));
    let credentials = Arc::new(PgConnectionCredentialVault::new(pool.clone(), KEY));
    let rules = Arc::new(PgAutomationRuleRepo::new(pool.clone()));
    let deliveries = Arc::new(PgWebhookDeliveryRepo::new(pool.clone()));
    let runs = Arc::new(PgAutomationRunRepo::new(pool.clone()));
    let incidents = Arc::new(PgIncidentRepo::new(pool.clone()));
    let releases = Arc::new(PgReleaseRepo::new(pool.clone()));
    let opswarden = connections
        .find_connection_by_service(team.id, "opswarden")
        .await
        .unwrap()
        .unwrap();
    let mut rule = AutomationRule::new(
        team.id,
        "Release opens an incident",
        opswarden.id,
        "release_created",
        serde_json::json!({}),
        "create_incident",
        None,
        serde_json::json!({"severity": "high", "title": "Release {{release_title}} created"}),
        manager.id,
    )
    .unwrap();
    rule.set_enabled(true);
    rules.insert_rule(&rule).await.unwrap();
    let release = Release::new(team.id, "v1.2.0", vec!["build".into()]).unwrap();
    releases.save_release(&release).await.unwrap();

    let use_case = DispatchInternalAutomationUseCase::new(InternalAutomationDependencies {
        connections,
        credentials,
        deliveries,
        rules,
        runs: runs.clone(),
        incidents: incidents.clone(),
        releases,
        notifier: Arc::new(common::DummyNotifier::default()),
        events: Arc::new(WsHub::new()),
        email_sender: Arc::new(opswarden_server::adapters::email::SmtpEmailSender::new()),
    });
    let event = release_created_event(&release);
    let delivery_id = format!("release:{}:created", release.id);
    let first = use_case
        .dispatch(DispatchInternalAutomationCommand {
            team_id: team.id,
            delivery_id: delivery_id.clone(),
            event: event.clone(),
        })
        .await
        .unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.rules_triggered, 1);
    let duplicate = use_case
        .dispatch(DispatchInternalAutomationCommand {
            team_id: team.id,
            delivery_id,
            event,
        })
        .await
        .unwrap();
    assert!(duplicate.duplicate);
    let persisted_runs = runs.list_runs_for_team(team.id, 10).await.unwrap();
    assert_eq!(persisted_runs.len(), 1);
    assert_eq!(persisted_runs[0].status, AutomationRunStatus::Succeeded);
    let persisted_incidents = incidents.list_incidents_for_team(team.id).await.unwrap();
    assert_eq!(persisted_incidents.len(), 1);
    assert_eq!(persisted_incidents[0].title, "Release v1.2.0 created");
    assert_eq!(
        persisted_runs[0].incident_id,
        Some(persisted_incidents[0].id)
    );
}

#[sqlx::test]
async fn postgres_generic_delivery_creates_one_incident_and_deduplicates(pool: PgPool) {
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let user = User::new(
        Email::new(format!("generic-pg-{}@test.local", Uuid::new_v4())).unwrap(),
        "hash",
    );
    users.save(&user).await.unwrap();
    let team = Team::new("Generic automation PG").unwrap();
    teams.save_team(&team).await.unwrap();

    let connections = Arc::new(PgServiceConnectionRepo::new(pool.clone()));
    let credentials = Arc::new(PgConnectionCredentialVault::new(pool.clone(), KEY));
    let rules = Arc::new(PgAutomationRuleRepo::new(pool.clone()));
    let deliveries = Arc::new(PgWebhookDeliveryRepo::new(pool.clone()));
    let runs = Arc::new(PgAutomationRunRepo::new(pool.clone()));
    let incidents = Arc::new(PgIncidentRepo::new(pool.clone()));
    let generic = ServiceConnection::new(team.id, "generic", user.id).unwrap();
    connections.insert_connection(&generic).await.unwrap();
    credentials
        .store_credential(
            generic.id,
            CredentialKind::WebhookSigningSecret,
            SIGNING_SECRET,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team.id,
        "Generic alert to Incident",
        generic.id,
        "generic_event",
        serde_json::json!({"event_type":"alert_firing", "source":"pg-monitor"}),
        "create_incident",
        None,
        serde_json::json!({"severity":"critical", "title":"{{source}}: {{title}}"}),
        user.id,
    )
    .unwrap();
    rule.set_enabled(true);
    rules.insert_rule(&rule).await.unwrap();

    let use_case = IngestTeamWebhookUseCase::new(TeamWebhookDependencies {
        connections: connections.clone(),
        credentials,
        verifier: Arc::new(HmacSha256Verifier),
        parser: Arc::new(GenericParser),
        deliveries,
        rules,
        runs: runs.clone(),
        incidents: incidents.clone(),
        releases: Arc::new(PgReleaseRepo::new(pool.clone())),
        notifier: Arc::new(common::DummyNotifier::default()),
        events: Arc::new(WsHub::new()),
        email_sender: Arc::new(opswarden_server::adapters::email::SmtpEmailSender::new()),
    });
    let command = || IngestTeamWebhookCommand {
        connection_id: generic.id,
        expected_service: "generic",
        provider_delivery_id: "pg-generic-delivery-42".into(),
        provider_event: "alert_firing".into(),
        signature: Some(SIGNING_SECRET.into()),
        body: GENERIC_BODY.to_vec(),
    };
    let first = use_case.ingest(command()).await.unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.rules_triggered, 1);
    let duplicate = use_case.ingest(command()).await.unwrap();
    assert!(duplicate.duplicate);

    let persisted_runs = runs.list_runs_for_team(team.id, 10).await.unwrap();
    assert_eq!(persisted_runs.len(), 1);
    assert_eq!(persisted_runs[0].status, AutomationRunStatus::Succeeded);
    let persisted_incidents = incidents.list_incidents_for_team(team.id).await.unwrap();
    assert_eq!(persisted_incidents.len(), 1);
    assert_eq!(
        persisted_incidents[0].title,
        "pg-monitor: Database unavailable"
    );
    assert_eq!(
        persisted_runs[0].incident_id,
        Some(persisted_incidents[0].id)
    );
}

fn command(connection_id: Uuid, signature: String) -> IngestTeamWebhookCommand {
    IngestTeamWebhookCommand {
        connection_id,
        expected_service: "github",
        provider_delivery_id: "pg-http-delivery-94".to_string(),
        provider_event: "workflow_run".to_string(),
        signature: Some(signature),
        body: FAILED_RUN.to_vec(),
    }
}
