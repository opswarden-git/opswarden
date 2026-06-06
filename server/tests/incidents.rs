mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::incident::{Incident, Severity};
use opswarden_server::domain::team::Role;
use opswarden_server::domain::timeline::TimelineEntry;
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
async fn list_timeline_entries_is_bounded_and_newest_first() {
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
                .uri(format!("/api/incidents/{}/timeline?limit=1", incident.id))
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
    assert_eq!(json["entries"].as_array().unwrap().len(), 1);
    assert_eq!(json["entries"][0]["content"], "Second update");
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
