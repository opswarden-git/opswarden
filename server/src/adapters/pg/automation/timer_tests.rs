use std::sync::Arc;

use chrono::TimeZone;
use serde_json::json;

use super::super::rule::PgAutomationRuleRepo;
use super::super::test_support::seed_team;
use super::*;
use crate::adapters::notify::HttpNotifier;
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
        rules: Arc::new(PgAutomationRuleRepo::new(pool.clone())),
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

    let expected_updated_at = rule.updated_at;
    rule.set_enabled(false);
    PgAutomationRuleRepo::new(pool.clone())
        .update_rule(&rule, expected_updated_at)
        .await
        .unwrap();
    assert!(!repo
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());
}

#[sqlx::test]
async fn concurrent_timer_edits_keep_only_the_winning_projection(pool: PgPool) {
    let (rule, _) = timer_rule(&pool, "concurrent-edit").await;
    let expected_updated_at = rule.updated_at;
    let mut ten_minutes = rule.clone();
    let mut fifteen_minutes = rule.clone();
    let mut ten_definition = ten_minutes.definition();
    ten_definition.trigger_config = json!({"minutes": "10", "timezone": "Europe/Paris"});
    ten_minutes.replace_definition(ten_definition).unwrap();
    let mut fifteen_definition = fifteen_minutes.definition();
    fifteen_definition.trigger_config = json!({"minutes": "15", "timezone": "Europe/Paris"});
    fifteen_minutes
        .replace_definition(fifteen_definition)
        .unwrap();

    let rules = PgAutomationRuleRepo::new(pool.clone());
    let (ten_result, fifteen_result) = tokio::join!(
        rules.update_rule(&ten_minutes, expected_updated_at),
        rules.update_rule(&fifteen_minutes, expected_updated_at),
    );

    let results = [ten_result, fifteen_result];
    assert_eq!(
        results.iter().filter(|result| result == &&Ok(true)).count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(DomainError::ConcurrentModification))
            .count(),
        1
    );
    let (config, rule_revision, interval, projection_revision) =
        sqlx::query_as::<_, (serde_json::Value, DateTime<Utc>, i32, DateTime<Utc>)>(
            r#"
            SELECT r.trigger_config, r.updated_at, s.interval_minutes, s.rule_updated_at
            FROM automation_rules r
            JOIN automation_timer_schedules s ON s.rule_id = r.id
            WHERE r.id = $1
            "#,
        )
        .bind(rule.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let configured_minutes = config["minutes"].as_str().unwrap().parse::<i32>().unwrap();
    assert_eq!(interval, configured_minutes);
    assert_eq!(projection_revision, rule_revision);
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
async fn one_broken_occurrence_does_not_stop_the_timer_batch(pool: PgPool) {
    let (first_rule, first_schedule) = timer_rule(&pool, "batch-first").await;
    let (second_rule, second_schedule) = timer_rule(&pool, "batch-second").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    for (rule, schedule) in [
        (&first_rule, &first_schedule),
        (&second_rule, &second_schedule),
    ] {
        assert!(timers
            .upsert_schedule(rule.id, schedule, now, rule.updated_at)
            .await
            .unwrap());
    }
    let failing_rule_id = first_rule.id.min(second_rule.id);
    sqlx::query("CREATE TABLE injected_timer_failures (rule_id uuid PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        CREATE FUNCTION fail_selected_timer_start() RETURNS trigger AS $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM injected_timer_failures WHERE rule_id = NEW.rule_id
            ) THEN
                RAISE EXCEPTION 'injected timer start failure';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        CREATE TRIGGER fail_selected_timer_start
        BEFORE UPDATE OF execution_started_at ON automation_timer_occurrences
        FOR EACH ROW EXECUTE FUNCTION fail_selected_timer_start()
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO injected_timer_failures (rule_id) VALUES ($1)")
        .bind(failing_rule_id)
        .execute(&pool)
        .await
        .unwrap();

    let result = timer_worker(&pool, timers).tick(now).await.unwrap();

    assert_eq!(result.claimed, 2);
    assert_eq!(result.succeeded, 1);
    assert_eq!(result.retried, 1);
    assert_eq!((result.failed, result.skipped), (0, 0));
    let successful_runs = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM automation_runs WHERE status = 'succeeded'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(successful_runs, 1);
}

#[sqlx::test]
async fn timer_completion_rolls_back_if_the_delivery_cannot_finish(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "finish-rollback").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = PgAutomationTimerRepo::new(pool.clone());
    assert!(timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap());
    let claim = timers.claim_due(now).await.unwrap().unwrap();
    let mut run = AutomationRun::new(claim.delivery_id, claim.rule_id);
    assert!(timers.start_execution(&claim, &run).await.unwrap());
    sqlx::query("UPDATE webhook_deliveries SET status = 'ignored' WHERE id = $1")
        .bind(claim.delivery_id)
        .execute(&pool)
        .await
        .unwrap();
    run.mark_succeeded(None).unwrap();

    assert!(!timers.finish_execution(&claim, &run).await.unwrap());

    let states = sqlx::query_as::<_, (String, String, Option<DateTime<Utc>>)>(
        r#"
        SELECT run.status, delivery.status, connection.last_delivery_at
        FROM automation_runs run
        JOIN webhook_deliveries delivery ON delivery.id = run.delivery_id
        JOIN service_connections connection ON connection.id = delivery.connection_id
        WHERE run.id = $1
        "#,
    )
    .bind(run.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(states, ("running".into(), "ignored".into(), None));
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
    let expected_updated_at = rule.updated_at;
    rule.set_enabled(false);
    PgAutomationRuleRepo::new(pool.clone())
        .update_rule(&rule, expected_updated_at)
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

include!("timer_extra_tests.rs");
