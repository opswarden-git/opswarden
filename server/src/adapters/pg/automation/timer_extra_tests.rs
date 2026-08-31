#[sqlx::test]
async fn unstarted_claim_is_abandoned_after_timeout(pool: PgPool) {
    let (rule, schedule) = timer_rule(&pool, "unstarted-abandon").await;
    let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    let timers = Arc::new(PgAutomationTimerRepo::new(pool.clone()));
    timers
        .upsert_schedule(rule.id, &schedule, now, rule.updated_at)
        .await
        .unwrap();
    assert!(timers.claim_due(now).await.unwrap().is_some());

    let worker = timer_worker(&pool, timers.clone());
    assert_eq!(
        worker.reconcile(now).await.unwrap().recovered,
        0
    );
    let result = worker
        .reconcile(now + chrono::Duration::seconds(31))
        .await
        .unwrap();
    assert_eq!(result.recovered, 1);
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
    assert_eq!(status, ("succeeded".to_string(), "processed".to_string()));
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
    let state = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        r#"
        SELECT r.status, r.error_code, d.status, c.last_error_code
        FROM automation_runs r
        JOIN webhook_deliveries d ON d.id = r.delivery_id
        JOIN service_connections c ON c.id = d.connection_id
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
            "failed".to_string(),
            Some("timer_worker_interrupted".to_string())
        )
    );
}
