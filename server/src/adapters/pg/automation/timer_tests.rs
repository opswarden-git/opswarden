use std::sync::Arc;

use chrono::TimeZone;
use serde_json::json;

use super::super::rule::PgAutomationRuleRepo;
use super::super::test_support::seed_team;
use super::*;
use crate::adapters::notify::HttpNotifier;
use crate::adapters::pg::automation::execution::{PgAutomationRunRepo, PgWebhookDeliveryRepo};
use crate::adapters::pg::automation::service_connection::{
    PgConnectionCredentialVault, PgServiceConnectionRepo,
};
use crate::adapters::pg::incident::PgIncidentRepo;
use crate::adapters::pg::release::PgReleaseRepo;
use crate::adapters::ws::WsHub;
use crate::app::automation::{TimerWorker, TimerWorkerDependencies};
use crate::domain::automation_config::AutomationRule;
use crate::domain::automation_timer::{DAILY_AT_KIND, EVERY_MINUTES_KIND};
use crate::ports::AutomationRuleRepo;

async fn timer_rule(pool: &PgPool, suffix: &str) -> (AutomationRule, TimerSchedule) {
    let (team_id, user_id) = seed_team(pool, suffix).await;
    let connection_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM service_connections WHERE team_id = $1 AND service = 'timer'",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let config = json!({"minutes": "5", "timezone": "Europe/Paris"});
    let schedule = TimerSchedule::from_config(EVERY_MINUTES_KIND, &config).unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("Timer {suffix}"),
        connection_id,
        EVERY_MINUTES_KIND,
        config,
        "create_incident",
        None,
        json!({"title": "Timer fired", "severity": "low"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    PgAutomationRuleRepo::new(pool.clone())
        .insert_rule(&rule)
        .await
        .unwrap();
    (rule, schedule)
}

fn timer_worker(pool: &PgPool, timers: Arc<PgAutomationTimerRepo>) -> TimerWorker {
    TimerWorker::new(TimerWorkerDependencies {
        timers,
        connections: Arc::new(PgServiceConnectionRepo::new(pool.clone())),
        credentials: Arc::new(PgConnectionCredentialVault::new(pool.clone(), [7; 32])),
        deliveries: Arc::new(PgWebhookDeliveryRepo::new(pool.clone())),
        rules: Arc::new(PgAutomationRuleRepo::new(pool.clone())),
        runs: Arc::new(PgAutomationRunRepo::new(pool.clone())),
        incidents: Arc::new(PgIncidentRepo::new(pool.clone())),
        releases: Arc::new(PgReleaseRepo::new(pool.clone())),
        notifier: Arc::new(HttpNotifier::new()),
        events: Arc::new(WsHub::new()),
        email_sender: Arc::new(crate::adapters::email::SmtpEmailSender::new()),
    })
}

#[sqlx::test]
async fn manager_membership_creates_one_internal_timer_connection(pool: PgPool) {
    let (team_id, _) = seed_team(&pool, "timer-connection").await;

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM service_connections WHERE team_id = $1 AND service = 'timer'",
    )
    .bind(team_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn projection_requires_current_enabled_timer_rule(pool: PgPool) {
    let (mut rule, schedule) = timer_rule(&pool, "projection").await;
    let repo = PgAutomationTimerRepo::new(pool.clone());
    let automatic_projection = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM automation_timer_schedules WHERE rule_id = $1",
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(automatic_projection, 1);

    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    assert!(repo
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());

    rule.set_enabled(false);
    PgAutomationRuleRepo::new(pool.clone())
        .update_rule(&rule)
        .await
        .unwrap();
    assert!(!repo
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());
}

#[sqlx::test]
async fn invalid_timer_rule_and_projection_roll_back_together(pool: PgPool) {
    let (team_id, user_id) = seed_team(&pool, "invalid-atomic").await;
    let connection_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM service_connections WHERE team_id = $1 AND service = 'timer'",
    )
    .bind(team_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "Invalid Timer",
        connection_id,
        DAILY_AT_KIND,
        json!({"time": "25:99", "timezone": "Europe/Paris"}),
        "create_incident",
        None,
        json!({"title": "Never created", "severity": "low"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);

    let error = PgAutomationRuleRepo::new(pool.clone())
        .insert_rule(&rule)
        .await
        .unwrap_err();
    assert_eq!(error, DomainError::InvalidTimerSchedule);
    let rule_count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM automation_rules WHERE id = $1")
            .bind(rule.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let schedule_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM automation_timer_schedules WHERE rule_id = $1",
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((rule_count, schedule_count), (0, 0));
}

#[sqlx::test]
async fn concurrent_workers_claim_one_occurrence_exactly_once(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "claim-race").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let repo = PgAutomationTimerRepo::new(pool.clone());
    assert!(repo
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());

    let first = PgAutomationTimerRepo::new(pool.clone());
    let second = PgAutomationTimerRepo::new(pool.clone());
    let (first_claim, second_claim) = tokio::join!(first.claim_due(now), second.claim_due(now));
    let claims = [first_claim.unwrap(), second_claim.unwrap()];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);

    let occurrence_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM automation_timer_occurrences WHERE rule_id = $1",
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let delivery_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM webhook_deliveries WHERE connection_id = $1",
    )
    .bind(rule.trigger_connection_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let next_run_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "SELECT next_run_at FROM automation_timer_schedules WHERE rule_id = $1",
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(occurrence_count, 1);
    assert_eq!(delivery_count, 1);
    assert_eq!(next_run_at, now + chrono::Duration::minutes(5));
}

#[sqlx::test]
async fn claimed_occurrence_executes_one_incident_and_one_successful_run(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "worker-e2e").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    assert!(timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());
    let worker = timer_worker(&pool, timers);

    let result = worker.tick(now).await.unwrap();
    assert_eq!(result.claimed, 1);
    assert_eq!(result.succeeded, 1);
    assert_eq!((result.failed, result.skipped), (0, 0));

    let incidents = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM incidents WHERE team_id = $1 AND title = 'Timer fired'",
    )
    .bind(rule.team_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let successful_runs = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM automation_runs WHERE rule_id = $1 AND status = 'succeeded'",
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((incidents, successful_runs), (1, 1));

    let repeated = worker.tick(now).await.unwrap();
    assert_eq!(repeated.claimed, 0);
    let incidents_after_repeat = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM incidents WHERE team_id = $1 AND title = 'Timer fired'",
    )
    .bind(rule.team_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(incidents_after_repeat, 1);
}

#[sqlx::test]
async fn reconciliation_recovers_an_unstarted_claim_once(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "recover-claim").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap();
    let claim = timers.claim_due(now).await.unwrap().unwrap();
    assert_eq!(claim.rule_id, rule.id);

    let worker = timer_worker(&pool, timers);
    let recovered = worker
        .reconcile(now + chrono::Duration::seconds(31))
        .await
        .unwrap();
    assert_eq!(recovered.recovered, 1);
    let repeated = worker
        .reconcile(now + chrono::Duration::seconds(62))
        .await
        .unwrap();
    assert_eq!(repeated.recovered, 0);
    let incidents = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM incidents WHERE team_id = $1 AND title = 'Timer fired'",
    )
    .bind(rule.team_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(incidents, 1);
}

#[sqlx::test]
async fn disabled_rule_turns_an_unstarted_claim_into_a_skipped_run(pool: PgPool) {
    let (mut rule, schedule) = timer_rule(&pool, "skip-disabled").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap();
    timers.claim_due(now).await.unwrap().unwrap();
    rule.set_enabled(false);
    PgAutomationRuleRepo::new(pool.clone())
        .update_rule(&rule)
        .await
        .unwrap();

    let worker = timer_worker(&pool, timers);
    worker
        .reconcile(now + chrono::Duration::seconds(31))
        .await
        .unwrap();
    let status = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT r.status, d.status
        FROM automation_runs r
        JOIN webhook_deliveries d ON d.id = r.delivery_id
        WHERE r.rule_id = $1
        "#,
    )
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, ("skipped".to_string(), "ignored".to_string()));
}

#[sqlx::test]
async fn stale_running_timer_run_is_failed_without_replay(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "stale-run").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap();
    let claim = timers.claim_due(now).await.unwrap().unwrap();
    let run = AutomationRun::new(claim.delivery_id, claim.rule_id);
    assert!(timers.start_execution(&claim, &run).await.unwrap());
    sqlx::query("UPDATE automation_runs SET started_at = $2 WHERE id = $1")
        .bind(run.id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

    let worker = timer_worker(&pool, timers);
    let result = worker
        .reconcile(now + chrono::Duration::minutes(6))
        .await
        .unwrap();
    assert_eq!(result.stale_runs_finalized, 1);
    let state = sqlx::query_as::<_, (String, Option<String>, String)>(
        r#"
        SELECT r.status, r.error_code, d.status
        FROM automation_runs r
        JOIN webhook_deliveries d ON d.id = r.delivery_id
        WHERE r.id = $1
        "#,
    )
    .bind(run.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        state,
        (
            "failed".to_string(),
            Some("timer_worker_interrupted".to_string()),
            "failed".to_string()
        )
    );
}
