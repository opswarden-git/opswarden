#[tokio::test]
async fn signed_delivery_creates_incident_and_durable_run_then_duplicate_is_noop() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let (connection, mut rule) = seed_automation(
        &ctx,
        team_id,
        SECRET_A,
        json!({"repository": "opswarden/app", "branch": "main"}),
        "create_incident",
    )
    .await;
    let mut definition = rule.definition();
    definition.reaction_config = json!({
        "severity": "critical",
        "title": "[{{repository}}] {{workflow}} failed"
    });
    let expected_updated_at = rule.updated_at;
    rule.replace_definition(definition).unwrap();
    assert!(ctx
        .automation_rules
        .update_rule(&rule, expected_updated_at)
        .await
        .unwrap());
    let (tx, mut rx) = mpsc::channel(256);
    ctx.events
        .register(Uuid::new_v4(), HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let receipt = json_body(response).await;
    assert_eq!(receipt["duplicate"], false);
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].title, "[opswarden/app] CI failed");
    assert!(incidents[0].description.contains("Branch: main"));
    assert_eq!(incidents[0].severity.to_string(), "critical");

    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, WebhookDeliveryStatus::Processed);
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AutomationRunStatus::Succeeded);
    assert_eq!(runs[0].incident_id, Some(incidents[0].id));
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "incident_created",
            "incident_id": incidents[0].id,
            "severity": "critical",
        })
    );
    let event: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({
            "type": "rule_triggered",
            "service": "github",
            "rule_name": "GitHub CI failed",
            "result": "incident_created",
            "incident_id": incidents[0].id,
        })
    );

    let persisted_connection = ctx
        .service_connections
        .find_connection_by_id(connection.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted_connection.verified_at.is_some());
    assert!(persisted_connection.last_delivery_at.is_some());
    assert_eq!(persisted_connection.last_error_code, None);

    let duplicate = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "delivery-42",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();
    let duplicate = json_body(duplicate).await;
    assert_eq!(duplicate["duplicate"], true);
    assert_eq!(duplicate["rules_triggered"], 0);
    assert_eq!(
        ctx.incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(ctx.automation_runs.all().len(), 1);
}
#[tokio::test]
async fn extended_github_actions_run_end_to_end_with_filters_and_templates() {
    let cases = [
        (
            "ci_succeeded",
            "workflow_run",
            SUCCEEDED_RUN,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "success"}),
            json!({"severity": "medium", "title": "{{workflow}} succeeded on {{repository}}"}),
            "CI succeeded on opswarden/app",
        ),
        (
            "tag_pushed",
            "push",
            NEW_TAG,
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            json!({"severity": "medium", "title": "Tag {{tag}} pushed by {{actor}}"}),
            "Tag v1.2.3 pushed by octocat",
        ),
        (
            "pr_merged",
            "pull_request",
            MERGED_PULL_REQUEST,
            json!({"repository": "opswarden/app", "branch": "main", "source_branch": "feature/opswarden"}),
            json!({"severity": "medium", "title": "PR #{{pull_request_number}} {{pull_request_title}}"}),
            "PR #42 Ship OpsWarden",
        ),
    ];

    for (index, (kind, provider_event, body, trigger_config, reaction_config, expected_title)) in
        cases.into_iter().enumerate()
    {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        let connection =
            seed_github_action(&ctx, team_id, kind, trigger_config, reaction_config).await;
        let response = ctx
            .app
            .clone()
            .oneshot(webhook_request(
                connection.id,
                &format!("extended-{index}"),
                provider_event,
                SECRET_A,
                body,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let receipt = json_body(response).await;
        assert_eq!(receipt["duplicate"], false);
        assert_eq!(receipt["rules_triggered"], 1);
        assert_eq!(receipt["rules_failed"], 0);
        let incidents = ctx
            .incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].title, expected_title);
        assert_eq!(
            ctx.webhook_deliveries.all()[0].status,
            WebhookDeliveryStatus::Processed
        );
        assert_eq!(
            ctx.automation_runs.all()[0].status,
            AutomationRunStatus::Succeeded
        );
    }
}

#[tokio::test]
async fn gitlab_actions_run_end_to_end_with_token_filters_templates_and_deduplication() {
    let cases = [
        (
            "ci_failed",
            "Pipeline Hook",
            GITLAB_FAILED_PIPELINE,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "failed"}),
            json!({"severity": "high", "title": "{{workflow}} failed on {{repository}}"}),
            "CI failed on opswarden/app",
        ),
        (
            "ci_succeeded",
            "Pipeline Hook",
            GITLAB_SUCCEEDED_PIPELINE,
            json!({"repository": "opswarden/app", "branch": "main", "conclusion": "success"}),
            json!({"severity": "medium", "title": "{{workflow}} succeeded on {{repository}}"}),
            "CI succeeded on opswarden/app",
        ),
        (
            "tag_pushed",
            "Tag Push Hook",
            GITLAB_NEW_TAG,
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            json!({"severity": "medium", "title": "Tag {{tag}} pushed by {{actor}}"}),
            "Tag v1.2.3 pushed by octocat",
        ),
    ];

    for (index, (kind, provider_event, body, trigger_config, reaction_config, expected_title)) in
        cases.into_iter().enumerate()
    {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        let connection =
            seed_gitlab_action(&ctx, team_id, kind, trigger_config, reaction_config).await;
        let delivery_id = format!("gitlab-{index}");
        let response = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &delivery_id,
                provider_event,
                GITLAB_TOKEN,
                body,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let receipt = json_body(response).await;
        assert_eq!(receipt["rules_triggered"], 1);
        assert_eq!(receipt["rules_failed"], 0);
        let incidents = ctx
            .incidents
            .list_incidents_for_team(team_id)
            .await
            .unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].title, expected_title);
        assert_eq!(
            ctx.automation_runs.all()[0].status,
            AutomationRunStatus::Succeeded
        );

        let duplicate = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &delivery_id,
                provider_event,
                GITLAB_TOKEN,
                body,
            ))
            .await
            .unwrap();
        assert_eq!(json_body(duplicate).await["duplicate"], true);
        assert_eq!(ctx.automation_runs.all().len(), 1);

        let rejected = ctx
            .app
            .clone()
            .oneshot(gitlab_webhook_request(
                connection.id,
                &format!("wrong-token-{index}"),
                provider_event,
                "wrong-token",
                body,
            ))
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(ctx.automation_runs.all().len(), 1);
    }
}

#[tokio::test]
async fn generic_json_runs_through_auth_filters_templates_durability_and_deduplication() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let connection = seed_generic_action(&ctx, team_id).await;

    let accepted = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-42",
            "deployment_failed",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    let receipt = json_body(accepted).await;
    assert_eq!(receipt["duplicate"], false);
    assert_eq!(receipt["rules_triggered"], 1);
    assert_eq!(receipt["rules_failed"], 0);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(
        incidents[0].title,
        "jury: Production deployment failed (deploy-42)"
    );
    assert_eq!(incidents[0].severity, Severity::Critical);
    assert!(incidents[0]
        .description
        .contains("Event type: deployment_failed"));
    assert!(incidents[0]
        .description
        .contains("Message: Health check timed out"));
    assert!(!incidents[0].description.contains("must-not-be-normalized"));
    assert_eq!(ctx.automation_runs.all().len(), 1);
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
    assert_eq!(ctx.webhook_deliveries.all().len(), 1);
    assert_eq!(
        ctx.webhook_deliveries.all()[0].status,
        WebhookDeliveryStatus::Processed
    );

    let duplicate = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-42",
            "deployment_failed",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(duplicate).await["duplicate"], true);
    assert_eq!(ctx.automation_runs.all().len(), 1);

    let filtered = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-43",
            "deployment_succeeded",
            GENERIC_TOKEN,
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(filtered).await["rules_triggered"], 0);
    assert_eq!(ctx.automation_runs.all().len(), 1);
    assert_eq!(ctx.webhook_deliveries.all().len(), 2);
    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(
        deliveries
            .iter()
            .find(|delivery| delivery.provider_delivery_id == "generic-delivery-43")
            .unwrap()
            .status,
        WebhookDeliveryStatus::Ignored
    );

    let rejected = ctx
        .app
        .clone()
        .oneshot(generic_webhook_request(
            connection.id,
            "generic-delivery-44",
            "deployment_failed",
            "wrong-token",
            GENERIC_EVENT,
        ))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(ctx.webhook_deliveries.all().len(), 2);
}

#[tokio::test]
async fn generic_endpoint_rejects_missing_headers_content_type_and_invalid_or_large_json() {
    let ctx = test_context();
    let connection_id = Uuid::new_v4();
    for request in [
        Request::builder()
            .method("POST")
            .uri(format!("/webhooks/generic/{connection_id}"))
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .unwrap(),
        Request::builder()
            .method("POST")
            .uri(format!("/webhooks/generic/{connection_id}"))
            .header("X-OpsWarden-Delivery", "delivery")
            .header("X-OpsWarden-Event", "event")
            .header("X-OpsWarden-Token", "token")
            .body(Body::from("{}"))
            .unwrap(),
        generic_webhook_request(connection_id, "delivery", "event", "token", "not-json"),
        generic_webhook_request(connection_id, "delivery", "event", "token", "[]"),
        generic_webhook_request(
            connection_id,
            "delivery",
            "event",
            "token",
            r#"{"severity":"urgent"}"#,
        ),
    ] {
        let response = ctx.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let oversized = format!(r#"{{"ignored":"{}"}}"#, "x".repeat(64 * 1024));
    let response = ctx
        .app
        .oneshot(generic_webhook_request(
            connection_id,
            "large",
            "event",
            "token",
            &oversized,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
