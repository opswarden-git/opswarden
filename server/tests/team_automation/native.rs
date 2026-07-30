#[tokio::test]
async fn release_creation_triggers_a_durable_native_opswarden_rule() {
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
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0].title, "Release v2.0.0 requires coordination");
    assert_eq!(incidents[0].severity.to_string(), "high");
    let deliveries = ctx.webhook_deliveries.all();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].provider_event, "release_created");
    assert_eq!(deliveries[0].status.to_string(), "processed");
    let runs = ctx.automation_runs.all();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status.to_string(), "succeeded");
    assert_eq!(runs[0].incident_id, Some(incidents[0].id));
}
