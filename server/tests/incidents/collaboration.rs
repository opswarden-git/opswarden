fn edit_request(incident_id: Uuid, entry_id: Uuid, content: &str) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(format!("/api/incidents/{incident_id}/timeline/{entry_id}"))
        .header("Authorization", "Bearer mock_jwt_token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({ "content": content }).to_string(),
        ))
        .unwrap()
}

fn react_request(incident_id: Uuid, entry_id: Uuid, emoji: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/api/incidents/{incident_id}/timeline/{entry_id}/reactions"
        ))
        .header("Authorization", "Bearer mock_jwt_token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({ "emoji": emoji }).to_string(),
        ))
        .unwrap()
}

async fn read_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn author_can_edit_their_timeline_entry() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let author = Uuid::nil(); // the mock-token user
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, author, Role::Responder);
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, author, "before").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let response = ctx
        .app
        .oneshot(edit_request(incident.id, entry.id, "after edit"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = read_json(response).await;
    assert_eq!(json["content"], "after edit");
    assert!(json["edited_at"].is_string());
}

#[tokio::test]
async fn non_author_member_cannot_edit_an_entry() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let author = Uuid::new_v4(); // someone else wrote it
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager); // requester is a member, not the author
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, author, "before").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let response = ctx
        .app
        .oneshot(edit_request(incident.id, entry.id, "hijack"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn editing_with_blank_content_is_rejected() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let author = Uuid::nil();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, author, Role::Responder);
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, author, "before").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let response = ctx
        .app
        .oneshot(edit_request(incident.id, entry.id, "   "))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn team_member_can_toggle_a_reaction() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let user = Uuid::nil();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, user, Role::Observer); // observers may react
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, Uuid::new_v4(), "react to me").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let first = ctx
        .app
        .clone()
        .oneshot(react_request(incident.id, entry.id, "👍"))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let j1 = read_json(first).await;
    assert_eq!(j1["reacted"], true);
    assert_eq!(j1["count"], 1);

    let second = ctx
        .app
        .oneshot(react_request(incident.id, entry.id, "👍"))
        .await
        .unwrap();
    let j2 = read_json(second).await;
    assert_eq!(j2["reacted"], false);
    assert_eq!(j2["count"], 0);
}

#[tokio::test]
async fn non_member_cannot_react() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    // The mock user is NOT seeded as a member.
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, Uuid::new_v4(), "react to me").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let response = ctx
        .app
        .oneshot(react_request(incident.id, entry.id, "👍"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn team_member_cannot_react_with_an_emoji_outside_the_catalog() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let user = Uuid::nil();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, user, Role::Observer);
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, Uuid::new_v4(), "react to me").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    let response = ctx
        .app
        .oneshot(react_request(incident.id, entry.id, "🔥"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = read_json(response).await;
    assert_eq!(json["code"], "invalid_reaction");
}

#[tokio::test]
async fn activity_includes_reaction_counts_and_reacted_flag() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let user = Uuid::nil();
    let incident = Incident::new(team_id, "DB latency", Severity::High).unwrap();
    ctx.teams.seed_member(team_id, user, Role::Observer);
    ctx.incidents.seed_incident(incident.clone());
    let entry = TimelineEntry::new(incident.id, Uuid::new_v4(), "react to me").unwrap();
    ctx.timeline.seed_entry(entry.clone());

    ctx.app
        .clone()
        .oneshot(react_request(incident.id, entry.id, "🎉"))
        .await
        .unwrap();

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/incidents/{}/activity", incident.id))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = read_json(response).await;
    let reactions = json["items"][0]["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "🎉");
    assert_eq!(reactions[0]["count"], 1);
    assert_eq!(reactions[0]["reacted"], true);
}
