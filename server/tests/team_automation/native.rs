#[tokio::test]
async fn release_creation_returns_before_native_rule_execution() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let opswarden = ServiceConnection::new_internal(team_id, "opswarden").unwrap();
    ctx.service_connections
        .insert_connection(&opswarden)
        .await
        .unwrap();

    let created = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Release created -> Incident",
                "trigger_connection_id": opswarden.id,
                "trigger_kind": "release_created",
                "trigger_config": {},
                "reaction_kind": "create_incident",
                "reaction_config": {
                    "severity": "high",
                    "title": "Release {{release_title}} requires coordination"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let rule = json_body(created).await;
    let enabled = ctx
        .app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!(
                "/api/teams/{team_id}/automation-rules/{}",
                rule["id"].as_str().unwrap()
            ),
            Some(json!({"enabled": true})),
        ))
        .await
        .unwrap();
    assert_eq!(enabled.status(), StatusCode::OK);

    let release = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            "/api/releases",
            Some(json!({
                "team_id": team_id,
                "title": "v2.0.0",
                "steps": ["build", "production"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(release.status(), StatusCode::CREATED);

    let incidents = ctx
        .incidents
        .list_incidents_for_team(team_id)
        .await
        .unwrap();
    assert!(incidents.is_empty());
    assert!(ctx.webhook_deliveries.all().is_empty());
    assert!(ctx.automation_runs.all().is_empty());
}
