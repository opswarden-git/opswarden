#[tokio::test]
async fn manager_can_assign_a_responder_to_an_incident() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let responder = Uuid::new_v4();
    let incident = Incident::new(team_id, "Primary DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, responder, Role::Responder);
    ctx.incidents.seed_incident(incident.clone());

    let payload = serde_json::json!({ "assignee_id": responder });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/incidents/{}/assign", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["assignee_id"], responder.to_string());
    assert_eq!(json["changed"], true);
}

#[tokio::test]
async fn observer_cannot_assign_a_responder() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let responder = Uuid::new_v4();
    let incident = Incident::new(team_id, "API saturation", Severity::Critical).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    ctx.teams.seed_member(team_id, responder, Role::Responder);
    ctx.incidents.seed_incident(incident.clone());

    let payload = serde_json::json!({ "assignee_id": responder });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/incidents/{}/assign", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn assigning_an_observer_is_unprocessable() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let observer = Uuid::new_v4();
    let incident = Incident::new(team_id, "Disk pressure", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, observer, Role::Observer);
    ctx.incidents.seed_incident(incident.clone());

    let payload = serde_json::json!({ "assignee_id": observer });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/incidents/{}/assign", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn posting_timeline_to_unknown_incident_returns_not_found() {
    let ctx = test_context();
    let payload = serde_json::json!({
        "content": "Investigating"
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/incidents/{}/timeline", Uuid::new_v4()))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
