#[tokio::test]
async fn create_incident_returns_created_for_team_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);

    let payload = serde_json::json!({
        "team_id": team_id,
        "title": "Primary DB latency",
        "severity": "high"
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/incidents")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["title"], "Primary DB latency");
    assert_eq!(json["status"], "open");
    assert_eq!(json["severity"], "high");
}

#[tokio::test]
async fn activity_reconstructs_system_events_and_human_notes() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    let responder = User::new(
        Email::new("responder@test.com").unwrap(),
        "unused-password-hash",
    );
    ctx.users.seed_user(responder.clone());
    ctx.teams
        .seed_member(team_id, responder.id, Role::Responder);

    let create = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/incidents")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "team_id": team_id,
                        "title": "Reconstruct me",
                        "description": "Durable activity",
                        "severity": "high"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_body = axum::body::to_bytes(create.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let incident_id = created["incident_id"].as_str().unwrap();

    for (uri, method, payload) in [
        (
            format!("/api/incidents/{incident_id}/timeline"),
            "POST",
            serde_json::json!({ "content": "Investigating database saturation" }),
        ),
        (
            format!("/api/incidents/{incident_id}/status"),
            "PUT",
            serde_json::json!({ "status": "acknowledged" }),
        ),
        (
            format!("/api/incidents/{incident_id}/assign"),
            "PUT",
            serde_json::json!({ "assignee_id": responder.id }),
        ),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("Authorization", "Bearer mock_jwt_token")
                    .header("Content-Type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_success());
    }

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{incident_id}/activity"))
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
    let items = json["items"].as_array().unwrap();

    assert_eq!(items.len(), 4);
    assert!(items
        .iter()
        .any(|item| item["type"] == "system_event" && item["kind"] == "created"));
    assert!(items.iter().any(|item| {
        item["type"] == "system_event"
            && item["kind"] == "status_changed"
            && item["actor"]["email"] == "existing@test.com"
    }));
    assert!(items.iter().any(|item| {
        item["type"] == "human_note"
            && item["content"] == "Investigating database saturation"
            && item["author"]["email"] == "existing@test.com"
    }));
    assert!(items.iter().any(|item| {
        item["type"] == "system_event"
            && item["kind"] == "assigned"
            && item["actor"]["email"] == "existing@test.com"
            && item["subject"]["email"] == "responder@test.com"
    }));
}

#[tokio::test]
async fn available_reactions_are_server_driven() {
    let ctx = test_context();
    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/reactions/available")
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
    assert_eq!(
        json["reactions"],
        serde_json::json!(["👍", "👀", "✅", "🚨", "❤️", "🎉"])
    );
}

#[tokio::test]
async fn observer_cannot_post_timeline_entries() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "API saturation", Severity::Critical).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    ctx.incidents.seed_incident(incident.clone());

    let payload = serde_json::json!({
        "content": "I should not be able to post"
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/incidents/{}/timeline", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(ctx.timeline.entries_for_incident(incident.id).is_empty());
}

#[tokio::test]
async fn activity_is_bounded_and_newest_first() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Ingress instability", Severity::High).unwrap();
    let author_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    ctx.teams.seed_member(team_id, author_id, Role::Responder);
    ctx.incidents.seed_incident(incident.clone());
    ctx.timeline
        .seed_entry(TimelineEntry::new(incident.id, author_id, "First update").unwrap());
    ctx.timeline
        .seed_entry(TimelineEntry::new(incident.id, author_id, "Second update").unwrap());

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}/activity?limit=1", incident.id))
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
    assert_eq!(json["items"][0]["content"], "Second update");
}

#[tokio::test]
async fn timeline_read_route_is_gone() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Ingress instability", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    ctx.incidents.seed_incident(incident.clone());

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}/timeline", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn change_status_rejects_unknown_status_values() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "Cache outage", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Responder);
    ctx.incidents.seed_incident(incident.clone());

    let payload = serde_json::json!({
        "status": "closed"
    });

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/incidents/{}/status", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

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
