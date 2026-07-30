#[tokio::test]
async fn manager_creates_updates_lists_and_deletes_a_disabled_by_default_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_github(&ctx, team_id).await;
    let connection_id = connection["id"].as_str().unwrap();

    let create = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "CI failed -> incident",
                "trigger_connection_id": connection_id,
                "trigger_kind": "ci_failed",
                "trigger_config": {"repository": "opswarden/app"},
                "reaction_kind": "create_incident",
                "reaction_config": {"severity": "high"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = json_body(create).await;
    assert_eq!(created["enabled"], false);
    let rule_id = created["id"].as_str().unwrap();

    let update = ctx
        .app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/teams/{team_id}/automation-rules/{rule_id}"),
            Some(json!({"name": "Production CI failed", "enabled": true})),
        ))
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let updated = json_body(update).await;
    assert_eq!(updated["name"], "Production CI failed");
    assert_eq!(updated["enabled"], true);

    let list = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/automation-rules"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(list).await.as_array().unwrap().len(), 1);

    let delete = ctx
        .app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/teams/{team_id}/automation-rules/{rule_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    assert!(ctx
        .automation_rules
        .list_rules_for_team(team_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn manager_can_create_every_catalogued_github_action_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_github(&ctx, team_id).await;
    let connection_id = connection["id"].as_str().unwrap();
    let cases = [
        (
            "ci_succeeded",
            json!({"repository": "opswarden/app", "branch": "main"}),
            "{{workflow}} succeeded",
        ),
        (
            "tag_pushed",
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
            "Tag {{tag}} pushed by {{actor}}",
        ),
        (
            "pr_merged",
            json!({"repository": "opswarden/app", "branch": "main", "source_branch": "feature/opswarden"}),
            "PR #{{pull_request_number}} {{pull_request_title}}",
        ),
    ];

    for (trigger_kind, trigger_config, title) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("GitHub {trigger_kind}"),
                    "trigger_connection_id": connection_id,
                    "trigger_kind": trigger_kind,
                    "trigger_config": trigger_config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high", "title": title}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{trigger_kind}");
        let rule = json_body(response).await;
        assert_eq!(rule["trigger_kind"], trigger_kind);
        assert_eq!(rule["enabled"], false);
    }
}

#[tokio::test]
async fn manager_can_create_every_catalogued_gitlab_action_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_gitlab(&ctx, team_id).await;
    let cases = [
        (
            "ci_failed",
            json!({"repository": "opswarden/app", "branch": "main"}),
        ),
        (
            "ci_succeeded",
            json!({"repository": "opswarden/app", "branch": "main"}),
        ),
        (
            "tag_pushed",
            json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
        ),
    ];
    for (trigger_kind, trigger_config) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("GitLab {trigger_kind}"),
                    "trigger_connection_id": connection["id"],
                    "trigger_kind": trigger_kind,
                    "trigger_config": trigger_config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high"}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{trigger_kind}");
        assert_eq!(json_body(response).await["trigger_kind"], trigger_kind);
    }
}

#[tokio::test]
async fn manager_can_create_bounded_timer_rules_and_invalid_schedule_is_rejected() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let timer = ServiceConnection::new_internal(team_id, "timer").unwrap();
    ctx.service_connections
        .insert_connection(&timer)
        .await
        .unwrap();

    for (name, kind, config) in [
        (
            "Daily handover",
            "daily_at",
            json!({"time": "09:30", "timezone": "Europe/Paris"}),
        ),
        (
            "Frequent check",
            "every_minutes",
            json!({"minutes": "15", "timezone": "UTC"}),
        ),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": name,
                    "trigger_connection_id": timer.id,
                    "trigger_kind": kind,
                    "trigger_config": config,
                    "reaction_kind": "create_incident",
                    "reaction_config": {"severity": "high"}
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{kind}");
    }

    let invalid = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Invalid timer",
                "trigger_connection_id": timer.id,
                "trigger_kind": "every_minutes",
                "trigger_config": {"minutes": "4", "timezone": "UTC"},
                "reaction_kind": "create_incident",
                "reaction_config": {"severity": "high"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(invalid).await["code"], "invalid_timer_schedule");
}

#[tokio::test]
async fn internal_automation_connections_cannot_be_deleted() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    for service in ["opswarden", "timer"] {
        let connection = ServiceConnection::new_internal(team_id, service).unwrap();
        ctx.service_connections
            .insert_connection(&connection)
            .await
            .unwrap();
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "DELETE",
                &format!("/api/teams/{team_id}/service-connections/{}", connection.id),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{service}");
        assert!(ctx
            .service_connections
            .find_connection_by_id(connection.id)
            .await
            .unwrap()
            .is_some());
    }
}

#[tokio::test]
async fn manager_can_create_a_filtered_generic_event_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = configure_generic(&ctx, team_id).await;
    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Generic deployment failed",
                "trigger_connection_id": connection["id"],
                "trigger_kind": "generic_event",
                "trigger_config": {
                    "event_type": "deployment_failed",
                    "source": "jury",
                    "severity": "critical"
                },
                "reaction_kind": "create_incident",
                "reaction_config": {
                    "severity": "critical",
                    "title": "{{source}}: {{title}} ({{external_id}})"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_body(response).await["enabled"], false);
}
