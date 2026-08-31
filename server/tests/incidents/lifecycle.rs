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
        .clone()
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

    let detail = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}", json["incident_id"].as_str().unwrap()))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(detail.into_body(), usize::MAX)
        .await
        .unwrap();
    let detail: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail["actions"]["transitions"], serde_json::json!(["acknowledged"]));
    assert_eq!(detail["actions"]["can_assign"], true);
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
            serde_json::json!({
                "content": "Investigating database saturation",
                "attachments": [{
                    "file_name": "runbook.txt",
                    "media_type": "text/plain",
                    "data_base64": "cnVuYm9vaw=="
                }]
            }),
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
        .clone()
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

    assert!(json["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature == "system_events"));
    assert!(json["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature == "attach_files"));

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
    let attachment_id = items
        .iter()
        .find(|item| item["type"] == "human_note")
        .unwrap()["attachments"][0]["id"]
        .as_str()
        .unwrap();
    let download = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/timeline-attachments/{attachment_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(download.into_body(), usize::MAX)
            .await
            .unwrap(),
        "runbook"
    );
    assert!(items.iter().any(|item| {
        item["type"] == "system_event"
            && item["kind"] == "assigned"
            && item["actor"]["email"] == "existing@test.com"
            && item["subject"]["email"] == "responder@test.com"
    }));
}

#[tokio::test]
async fn incident_timeline_accepts_an_attachment_above_axums_default_body_limit() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    let incident = Incident::new(team_id, "Large runbook", Severity::High).unwrap();
    ctx.incidents.seed_incident(incident.clone());
    let content = vec![b'x'; 3 * 1024 * 1024];
    let payload = serde_json::json!({
        "content": "",
        "attachments": [{
            "file_name": "runbook.txt",
            "media_type": "text/plain",
            "data_base64": base64::engine::general_purpose::STANDARD.encode(content)
        }]
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

    assert_eq!(response.status(), StatusCode::CREATED);
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


