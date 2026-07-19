mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::incident::{Incident, Severity};
use opswarden_server::domain::team::Role;
use opswarden_server::domain::timeline::TimelineEntry;
use opswarden_server::domain::user::{Email, User};
use tower::ServiceExt;
use uuid::Uuid;

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
                .uri("/api/incidents/reactions/available")
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
        serde_json::json!(["👍", "👀", "✅", "🚨"])
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

// --- RTC 2: timeline edit + reactions ---

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
        .oneshot(react_request(incident.id, entry.id, "🔥"))
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
    assert_eq!(reactions[0]["emoji"], "🔥");
    assert_eq!(reactions[0]["count"], 1);
    assert_eq!(reactions[0]["reacted"], true);
}
