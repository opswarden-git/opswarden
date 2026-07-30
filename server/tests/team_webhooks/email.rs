#[tokio::test]
async fn email_reaction_interpolates_templates_and_persists_a_successful_run() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, email, _) = seed_email_automation(
        &ctx,
        team_id,
        json!({
            "to": "oncall@example.com",
            "subject": "CI failed: {{workflow}}",
            "body": "{{repository}} broke on {{branch}}"
        }),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "email-delivery-1",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);

    let sent = ctx.email_sender.sent();
    assert_eq!(sent.len(), 1);
    let (config, message) = &sent[0];
    // The five decrypted credentials must land in the right fields.
    assert_eq!(config.host, "smtp.example.com");
    assert_eq!(config.port, 587);
    assert_eq!(config.username, "opswarden");
    assert_eq!(config.password, "smtp-password");
    assert_eq!(config.from, "alerts@example.com");
    assert_eq!(message.to, "oncall@example.com");
    assert_eq!(message.subject, "CI failed: CI");
    assert_eq!(message.body, "opswarden/app broke on main");

    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    let persisted = ctx
        .service_connections
        .find_connection_by_id(email.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.verified_at.is_some());
    assert_eq!(persisted.last_error_code, None);
}

#[tokio::test]
async fn email_reaction_falls_back_to_the_event_summary_without_templates() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, _, _) =
        seed_email_automation(&ctx, team_id, json!({"to": "oncall@example.com"})).await;

    ctx.app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "email-delivery-2",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    let sent = ctx.email_sender.sent();
    assert_eq!(sent.len(), 1);
    assert!(!sent[0].1.subject.is_empty());
    assert_eq!(sent[0].1.subject, sent[0].1.body);
}

#[tokio::test]
async fn a_rejected_recipient_is_recorded_as_such_and_not_as_a_transport_outage() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (github, email, _) =
        seed_email_automation(&ctx, team_id, json!({"to": "oncall@example.com"})).await;
    ctx.email_sender
        .fail_with(|| DomainError::InvalidEmailRecipient);

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "email-delivery-3",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(response).await["rules_failed"], 1);

    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Failed);
    let persisted = ctx
        .service_connections
        .find_connection_by_id(email.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.last_error_code.as_deref(),
        Some("invalid_email_recipient")
    );
}

#[tokio::test]
async fn an_email_reaction_pointing_at_another_service_is_refused() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (github, http, _) = seed_http_automation(&ctx, team_id, SECRET_A).await;
    // Same shape as a real rule, except the reaction targets the HTTP connection.
    let mut rule = AutomationRule::new(
        team_id,
        "Email rule bound to an HTTP connection",
        github.id,
        "ci_failed",
        json!({}),
        "email_notify",
        Some(http.id),
        json!({"to": "oncall@example.com"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();

    ctx.app
        .clone()
        .oneshot(webhook_request(
            github.id,
            "email-delivery-4",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert!(ctx.email_sender.sent().is_empty());
    let failed = ctx
        .automation_runs
        .all()
        .into_iter()
        .filter(|run| run.status == AutomationRunStatus::Failed)
        .count();
    assert_eq!(failed, 1);
}
