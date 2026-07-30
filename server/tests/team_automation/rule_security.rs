#[tokio::test]
async fn manager_can_create_every_catalogued_native_opswarden_reaction_rule() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let github = configure_github(&ctx, team_id).await;
    let release_id = Uuid::new_v4();
    let incident_id = Uuid::new_v4();
    let cases = [
        (
            "validate_release_step",
            json!({"release_id": release_id, "step": "build"}),
        ),
        (
            "block_release",
            json!({
                "release_id": release_id,
                "severity": "critical",
                "title": "{{workflow}} blocks the release"
            }),
        ),
        ("escalate_incident", json!({"incident_id": incident_id})),
    ];

    for (reaction_kind, reaction_config) in cases {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/teams/{team_id}/automation-rules"),
                Some(json!({
                    "name": format!("CI failed to {reaction_kind}"),
                    "trigger_connection_id": github["id"],
                    "trigger_kind": "ci_failed",
                    "trigger_config": {},
                    "reaction_kind": reaction_kind,
                    "reaction_config": reaction_config
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "{reaction_kind}");
        let rule = json_body(response).await;
        assert_eq!(rule["reaction_kind"], reaction_kind);
        assert_eq!(rule["enabled"], false);
    }
}

#[tokio::test]
async fn http_rule_requires_its_own_team_connection_and_a_catalog_bounded_payload() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let github = configure_github(&ctx, team_id).await;
    let http = configure_http(&ctx, team_id).await;

    let valid = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "CI failed -> HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {
                    "message": "{{workflow}} failed on {{repository}}"
                }
            })),
        ))
        .await
        .unwrap();
    assert_eq!(valid.status(), StatusCode::CREATED);
    assert_eq!(json_body(valid).await["enabled"], false);

    let configurable_payload = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Unsafe customizable HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {"headers": {"Authorization": "secret"}}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(configurable_payload.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(configurable_payload).await["code"],
        "invalid_automation_rule"
    );

    let unknown_variable = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Secret template",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": http["id"],
                "reaction_config": {"message": "{{oauth_access_token}}"}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(unknown_variable.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(unknown_variable).await["code"],
        "invalid_automation_rule"
    );

    let team_b = Uuid::new_v4();
    let foreign_http = ServiceConnection::new(team_b, "http", Uuid::new_v4()).unwrap();
    ctx.service_connections
        .insert_connection(&foreign_http)
        .await
        .unwrap();
    let cross_team = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/automation-rules"),
            Some(json!({
                "name": "Cross-Team HTTP",
                "trigger_connection_id": github["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {},
                "reaction_kind": "http_notify",
                "reaction_connection_id": foreign_http.id,
                "reaction_config": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(cross_team.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json_body(cross_team).await["code"],
        "service_connection_not_found"
    );
}

#[tokio::test]
async fn cross_team_trigger_and_secret_shaped_rule_config_are_rejected() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    ctx.teams.seed_member(team_a, REQUESTER, Role::Manager);
    let connection_b = ServiceConnection::new(team_b, "github", Uuid::new_v4()).unwrap();
    ctx.service_connections
        .insert_connection(&connection_b)
        .await
        .unwrap();

    let base = json!({
        "name": "bad rule",
        "trigger_connection_id": connection_b.id,
        "trigger_kind": "ci_failed",
        "trigger_config": {},
        "reaction_kind": "create_incident",
        "reaction_config": {}
    });
    let cross_team = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_a}/automation-rules"),
            Some(base),
        ))
        .await
        .unwrap();
    assert_eq!(cross_team.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json_body(cross_team).await["code"],
        "service_connection_not_found"
    );

    let own_connection = configure_github(&ctx, team_a).await;
    let leaky = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_a}/automation-rules"),
            Some(json!({
                "name": "leaky rule",
                "trigger_connection_id": own_connection["id"],
                "trigger_kind": "ci_failed",
                "trigger_config": {"access_token": "must-not-be-persisted"},
                "reaction_kind": "create_incident",
                "reaction_config": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(leaky.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(leaky).await["code"], "invalid_automation_rule");
}

#[tokio::test]
async fn team_automation_routes_require_authentication() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/api/teams/{team_id}/automation-rules"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
