use serde_json::json;

use super::super::rule::PgAutomationRuleRepo;
use super::super::service_connection::PgServiceConnectionRepo;
use super::super::test_support::seed_team;
use super::*;
use crate::domain::automation_config::{AutomationRule, ServiceConnection};
use crate::ports::{AutomationRuleRepo, ServiceConnectionRepo};

async fn setup_rule(pool: &PgPool, suffix: &str) -> (Uuid, ServiceConnection, AutomationRule) {
    let (team_id, user_id) = seed_team(pool, suffix).await;
    let connections = PgServiceConnectionRepo::new(pool.clone());
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    connections.insert_connection(&connection).await.unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("Rule {suffix}"),
        connection.id,
        "ci_failed",
        json!({}),
        "create_incident",
        None,
        json!({"severity": "high"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    PgAutomationRuleRepo::new(pool.clone())
        .insert_rule(&rule)
        .await
        .unwrap();
    (team_id, connection, rule)
}

#[sqlx::test]
async fn provider_delivery_is_reserved_once_per_connection(pool: PgPool) {
    let (team_a, connection_a, _) = setup_rule(&pool, "delivery-a").await;
    let (team_b, connection_b, _) = setup_rule(&pool, "delivery-b").await;
    let repo = PgWebhookDeliveryRepo::new(pool);
    let delivery_a =
        WebhookDelivery::new(connection_a.id, "github-delivery-42", "workflow_run").unwrap();
    let delivery_b =
        WebhookDelivery::new(connection_b.id, "github-delivery-42", "workflow_run").unwrap();

    let claim_a = repo.claim_delivery(&delivery_a).await.unwrap().unwrap();
    assert!(repo.claim_delivery(&delivery_a).await.unwrap().is_none());
    assert!(repo.claim_delivery(&delivery_b).await.unwrap().is_some());

    let mut processed_a = delivery_a.clone();
    processed_a.mark_processed().unwrap();
    assert!(repo
        .complete_claimed_delivery(&processed_a, claim_a)
        .await
        .unwrap());
    assert!(!repo
        .complete_claimed_delivery(&processed_a, claim_a)
        .await
        .unwrap());

    let mut already_terminal =
        WebhookDelivery::new(connection_a.id, "already-terminal", "workflow_run").unwrap();
    already_terminal.mark_ignored().unwrap();
    assert_eq!(
        repo.claim_delivery(&already_terminal).await.unwrap_err(),
        DomainError::InvalidWebhookDelivery
    );
    assert_eq!(
        repo.list_deliveries_for_team(team_a, 20).await.unwrap(),
        vec![processed_a]
    );
    assert_eq!(
        repo.list_deliveries_for_team(team_b, 20).await.unwrap(),
        vec![delivery_b]
    );
}

#[sqlx::test]
async fn expired_delivery_claim_is_recoverable_and_fences_stale_worker(pool: PgPool) {
    let (_, connection, rule) = setup_rule(&pool, "delivery-reclaim").await;
    let repo = PgWebhookDeliveryRepo::new(pool.clone());
    let original =
        WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
    let stale_claim = repo.claim_delivery(&original).await.unwrap().unwrap();
    let runs = PgAutomationRunRepo::new(pool.clone());
    runs.reserve_run(
        &AutomationRun::new(stale_claim.delivery_id, rule.id),
        stale_claim,
    )
    .await
    .unwrap();

    sqlx::query(
        "UPDATE webhook_deliveries SET claim_expires_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(stale_claim.delivery_id)
    .execute(&pool)
    .await
    .unwrap();

    let mut retry =
        WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
    let active_claim = repo.claim_delivery(&retry).await.unwrap().unwrap();
    assert_eq!(active_claim.delivery_id, stale_claim.delivery_id);
    assert_ne!(active_claim.token, stale_claim.token);
    assert_eq!(
        runs.interrupt_running_for_delivery(stale_claim)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        runs.interrupt_running_for_delivery(active_claim)
            .await
            .unwrap(),
        1
    );
    retry.id = active_claim.delivery_id;
    retry.mark_processed().unwrap();

    assert!(!repo
        .complete_claimed_delivery(&retry, stale_claim)
        .await
        .unwrap());
    assert!(repo
        .complete_claimed_delivery(&retry, active_claim)
        .await
        .unwrap());
    let terminal_retry =
        WebhookDelivery::new(connection.id, "abandoned-delivery", "workflow_run").unwrap();
    assert!(repo
        .claim_delivery(&terminal_retry)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test]
async fn concurrent_delivery_claim_has_a_single_owner(pool: PgPool) {
    let (_, connection, _) = setup_rule(&pool, "delivery-concurrent").await;
    let delivery =
        WebhookDelivery::new(connection.id, "concurrent-delivery", "workflow_run").unwrap();
    let other = delivery.clone();
    let first_repo = PgWebhookDeliveryRepo::new(pool.clone());
    let second_repo = PgWebhookDeliveryRepo::new(pool);

    let (first, second) = tokio::join!(
        first_repo.claim_delivery(&delivery),
        second_repo.claim_delivery(&other)
    );
    assert_eq!(
        usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()),
        1
    );
}

#[sqlx::test]
async fn runs_persist_terminal_state_and_remain_team_scoped(pool: PgPool) {
    let (team_a, connection_a, rule_a) = setup_rule(&pool, "run-a").await;
    let (team_b, _, _) = setup_rule(&pool, "run-b").await;
    let deliveries = PgWebhookDeliveryRepo::new(pool.clone());
    let delivery =
        WebhookDelivery::new(connection_a.id, "run-delivery", "workflow_run").unwrap();
    let claim = deliveries.claim_delivery(&delivery).await.unwrap().unwrap();

    let runs = PgAutomationRunRepo::new(pool);
    let mut run = AutomationRun::new(delivery.id, rule_a.id);
    assert_eq!(
        runs.reserve_run(&run, claim).await.unwrap(),
        AutomationRunReservation::New(run.clone())
    );
    assert_eq!(
        runs.reserve_run(&AutomationRun::new(delivery.id, rule_a.id), claim)
            .await
            .unwrap(),
        AutomationRunReservation::Existing(run.clone())
    );
    assert_eq!(runs.list_runs_for_team(team_b, 20).await.unwrap(), vec![]);

    run.mark_succeeded(None).unwrap();
    assert!(runs.update_run(&run).await.unwrap());
    assert!(!runs.update_run(&run).await.unwrap());
    assert_eq!(
        runs.list_runs_for_team(team_a, 20).await.unwrap(),
        vec![run]
    );
}

#[sqlx::test]
async fn run_cannot_pair_delivery_with_rule_from_another_connection(pool: PgPool) {
    let (_, connection_a, _) = setup_rule(&pool, "run-cross-a").await;
    let (_, _, rule_b) = setup_rule(&pool, "run-cross-b").await;
    let deliveries = PgWebhookDeliveryRepo::new(pool.clone());
    let delivery =
        WebhookDelivery::new(connection_a.id, "cross-delivery", "workflow_run").unwrap();
    let claim = deliveries.claim_delivery(&delivery).await.unwrap().unwrap();

    let runs = PgAutomationRunRepo::new(pool);
    let run = AutomationRun::new(delivery.id, rule_b.id);
    assert_eq!(
        runs.reserve_run(&run, claim).await.unwrap_err(),
        DomainError::Storage
    );
}
