#[tokio::test]
async fn signed_delivery_notifies_http_once_and_persists_a_successful_run() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, http, _) = seed_http_automation(&ctx, team_id, SECRET_A).await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "http-delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(
        ctx.notifier.calls()[0].0,
        "https://hooks.example.com/opswarden-secret"
    );
    assert_eq!(ctx.notifier.calls()[0].1, "Alert: CI / opswarden/app");
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(runs[0].incident_id, None);
    let persisted_http = ctx
        .service_connections
        .find_connection_by_id(http.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted_http.verified_at.is_some());
    assert!(persisted_http.last_delivery_at.is_none());

    let duplicate = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "http-delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(duplicate).await["duplicate"], true);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(ctx.automation_runs.all().len(), 1);
}
#[tokio::test]
async fn failed_http_reaction_does_not_block_the_opswarden_reaction() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, http, _) = seed_http_automation(&ctx, team_id, SECRET_A).await;
    let mut opswarden_rule = AutomationRule::new(
        team_id,
        "GitHub CI failed to OpsWarden",
        github.id,
        "ci_failed",
        json!({}),
        "create_incident",
        None,
        json!({"severity": "high"}),
        Uuid::new_v4(),
    )
    .unwrap();
    opswarden_rule.set_enabled(true);
    ctx.automation_rules
        .insert_rule(&opswarden_rule)
        .await
        .unwrap();
    ctx.notifier.fail_requests();

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "mixed-delivery",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 1);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap()
            .len(),
        1
    );
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 2);
    assert!(runs
        .iter()
        .any(|run| run.status == AutomationRunStatus::Succeeded));
    assert!(runs.iter().any(|run| {
        run.status == AutomationRunStatus::Failed
            && run.error_code.as_deref() == Some("reaction_http_5xx")
    }));
    let persisted_http = ctx
        .service_connections
        .find_connection_by_id(http.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted_http.last_error_code.as_deref(),
        Some("reaction_http_5xx")
    );
}

#[tokio::test]
async fn connection_secret_and_rules_are_isolated_between_teams() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    let (connection_a, _) =
        seed_automation(&ctx, team_a, SECRET_A, json!({}), "create_incident").await;
    seed_automation(&ctx, team_b, SECRET_B, json!({}), "create_incident").await;

    let wrong_secret = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection_a.id,
            "wrong-secret",
            "workflow_run",
            SECRET_B,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(wrong_secret.status(), StatusCode::UNAUTHORIZED);
    assert!(ctx.webhook_deliveries.all().is_empty());

    let accepted = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection_a.id,
            "team-a-delivery",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_a)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(ctx
        .incidents
        .list_incidents_for_team(team_b)
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn signed_ping_verifies_connection_without_running_rules() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (connection, _) =
        seed_automation(&ctx, team_id, SECRET_A, json!({}), "create_incident").await;
    let ping = r#"{"zen":"Keep it logically awesome."}"#;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "ping-1",
            "ping",
            SECRET_A,
            ping,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(
        ctx.webhook_deliveries.all()[0].status,
        WebhookDeliveryStatus::Ignored
    );
    assert!(ctx.automation_runs.all().is_empty());
    assert!(ctx
        .service_connections
        .find_connection_by_id(connection.id)
        .await
        .unwrap()
        .unwrap()
        .verified_at
        .is_some());
}

#[tokio::test]
async fn filter_mismatch_creates_no_run_and_unsupported_reaction_records_failure() {
    let ctx = test_context();
    let filtered_team = Uuid::new_v4();
    let (filtered_connection, _) = seed_automation(
        &ctx,
        filtered_team,
        SECRET_A,
        json!({"repository": "another/project"}),
        "create_incident",
    )
    .await;
    let ignored = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            filtered_connection.id,
            "filtered",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(ignored).await["rules_triggered"], 0);
    assert!(ctx.automation_runs.all().is_empty());

    let failing_team = Uuid::new_v4();
    let (failing_connection, _) =
        seed_automation(&ctx, failing_team, SECRET_B, json!({}), "http_notify").await;
    let (tx, mut rx) = mpsc::channel(256);
    ctx.events
        .register(Uuid::new_v4(), HashSet::from([failing_team]), tx);
    while rx.try_recv().is_ok() {}
    let failed = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            failing_connection.id,
            "unsupported",
            "workflow_run",
            SECRET_B,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(failed).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["rules_failed"], 1);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Failed);
    assert_eq!(
        runs[0].error_code.as_deref(),
        Some("invalid_automation_rule")
    );
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "rule_failed",
            "service": "github",
            "rule_name": "GitHub CI failed",
            "error": "invalid_automation_rule",
        })
    );
    assert!(ctx
        .incidents
        .list_incidents_for_team(failing_team)
        .await
        .unwrap()
        .is_empty());
}
