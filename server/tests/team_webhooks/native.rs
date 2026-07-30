#[tokio::test]
async fn github_event_validates_the_next_release_step_and_records_the_run() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let release =
        Release::new(team_id, "v1.1.0", vec!["build".into(), "production".into()]).unwrap();
    ctx.releases.save_release(&release).await.unwrap();
    let (connection, rule) = seed_github_reaction(
        &ctx,
        team_id,
        "validate_release_step",
        json!({"release_id": release.id, "step": "build"}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "validate-release-step",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let stored = ctx
        .releases
        .find_release_by_id(release.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base_state, ReleaseState::InProgress);
    assert_eq!(stored.steps[0].validated_by, rule.created_by);
    assert!(stored.steps[0].validated_at.is_some());
    assert!(stored.steps[1].validated_at.is_none());
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
}

#[tokio::test]
async fn github_event_blocks_an_in_progress_release_with_a_linked_incident() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let mut release =
        Release::new(team_id, "v1.1.0", vec!["build".into(), "production".into()]).unwrap();
    release
        .validate_step("build", Uuid::new_v4(), false)
        .unwrap();
    ctx.releases.save_release(&release).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        team_id,
        "block_release",
        json!({
            "release_id": release.id,
            "severity": "critical",
            "title": "{{workflow}} blocks release"
        }),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "block-release",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let linked = ctx
        .releases
        .list_linked_incident_ids(release.id)
        .await
        .unwrap();
    assert_eq!(linked.len(), 1);
    let incident = ctx
        .incidents
        .find_incident_by_id(linked[0])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(incident.title, "CI blocks release");
    assert_eq!(incident.severity, Severity::Critical);
    assert_eq!(
        release.effective_state(
            ctx.releases
                .count_active_linked_incidents(release.id)
                .await
                .unwrap()
                > 0
        ),
        ReleaseState::Blocked
    );
    let run = &ctx.automation_runs.all()[0];
    assert_eq!(run.status, AutomationRunStatus::Succeeded);
    assert_eq!(run.incident_id, Some(incident.id));
}

#[tokio::test]
async fn github_event_escalates_an_acknowledged_incident_with_an_audit_event() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let mut incident = Incident::new(team_id, "Database latency", Severity::High).unwrap();
    incident.acknowledge().unwrap();
    ctx.incidents.save_incident(&incident).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        team_id,
        "escalate_incident",
        json!({"incident_id": incident.id}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "escalate-incident",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(response).await["rules_triggered"], 1);
    let stored = ctx
        .incidents
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, IncidentStatus::Escalated);
    let events = ctx
        .incidents
        .list_events_for_incident(incident.id, 10)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Succeeded
    );
}

#[tokio::test]
async fn native_reactions_cannot_mutate_another_teams_release() {
    let ctx = test_context();
    let source_team = Uuid::new_v4();
    let foreign_release = Release::new(
        Uuid::new_v4(),
        "foreign",
        vec!["build".into(), "production".into()],
    )
    .unwrap();
    ctx.releases.save_release(&foreign_release).await.unwrap();
    let (connection, _) = seed_github_reaction(
        &ctx,
        source_team,
        "validate_release_step",
        json!({"release_id": foreign_release.id, "step": "build"}),
    )
    .await;

    let response = ctx
        .app
        .clone()
        .oneshot(webhook_request(
            connection.id,
            "foreign-release",
            "workflow_run",
            SECRET_A,
            FAILED_RUN,
        ))
        .await
        .unwrap();

    let receipt = json_body(response).await;
    assert_eq!(receipt["rules_triggered"], 0);
    assert_eq!(receipt["rules_failed"], 1);
    let stored = ctx
        .releases
        .find_release_by_id(foreign_release.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.base_state, ReleaseState::Created);
    assert!(stored.steps.iter().all(|step| !step.is_validated()));
    assert_eq!(
        ctx.automation_runs.all()[0].status,
        AutomationRunStatus::Failed
    );
}

#[tokio::test]
async fn provider_headers_are_required_and_body_is_limited() {
    let ctx = test_context();
    let missing_header = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/webhooks/github/{}", Uuid::new_v4()))
                .header("X-GitHub-Event", "workflow_run")
                .body(Body::from(FAILED_RUN))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_header.status(), StatusCode::BAD_REQUEST);

    let oversized = "x".repeat(1024 * 1024 + 1);
    let too_large = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/webhooks/github/{}", Uuid::new_v4()))
                .header("X-GitHub-Delivery", "large")
                .header("X-GitHub-Event", "workflow_run")
                .header("X-Hub-Signature-256", "sha256=deadbeef")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(too_large.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
