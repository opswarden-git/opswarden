#[tokio::test]
async fn manager_can_delete_incident() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Incident to delete", Severity::High).unwrap();
    let requester = Uuid::nil();

    ctx.teams.seed_member(team_id, requester, Role::Manager);
    ctx.incidents.seed_incident(incident.clone());

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/incidents/{}", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn list_incidents_returns_team_incidents_for_a_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    ctx.incidents.seed_incident(incident.clone());

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents?team_id={team_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let incidents = json["items"].as_array().unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0]["title"], "DB latency");
    assert_eq!(incidents[0]["severity"], "high");
    assert_eq!(incidents[0]["status"], "open");
    assert_eq!(json["counts"]["all"], 1);
    assert_eq!(json["counts"]["open"], 1);
}

#[tokio::test]
async fn list_incidents_applies_url_filters_without_losing_view_counts() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();
    let responder = Uuid::new_v4();
    let mut matching =
        Incident::new(team_id, "Primary database latency", Severity::Critical).unwrap();
    matching.assign(responder);
    let other = Incident::new(team_id, "API timeout", Severity::Low).unwrap();
    ctx.teams.seed_member(team_id, requester, Role::Observer);
    ctx.teams.seed_member(team_id, responder, Role::Responder);
    ctx.incidents.seed_incident(matching);
    ctx.incidents.seed_incident(other);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/incidents?team_id={team_id}&status=open&severity=critical&assignee={responder}&q=database&sort=severity"
                ))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        json["items"][0]["assignee"]["user_id"],
        responder.to_string()
    );
    assert_eq!(json["counts"]["all"], 2);
    assert_eq!(json["counts"]["open"], 2);
}

#[tokio::test]
async fn list_incidents_is_forbidden_for_a_non_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    // No membership seeded for the mock user (Uuid::nil()).

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents?team_id={team_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_incident_returns_detail_for_a_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Cache outage", Severity::Critical).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Responder);
    ctx.incidents.seed_incident(incident.clone());

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["incident_id"], incident.id.to_string());
    assert_eq!(json["severity"], "critical");
    assert_eq!(json["status"], "open");
}

#[tokio::test]
async fn get_incident_is_forbidden_for_a_non_member() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Cache outage", Severity::Critical).unwrap();
    ctx.incidents.seed_incident(incident.clone());
    // mock user is not a member of the incident's team.

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// --- Timeline edit and reactions ---
