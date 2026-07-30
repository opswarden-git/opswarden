fn set_role_request(team_id: Uuid, user_id: Uuid, role: &str) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(format!("/api/teams/{team_id}/members/{user_id}/role"))
        .header("Authorization", "Bearer mock_jwt_token")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::json!({ "role": role }).to_string()))
        .unwrap()
}

#[tokio::test]
async fn manager_promotes_a_member_to_responder() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let manager = Uuid::nil();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, manager, Role::Manager);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(set_role_request(team_id, member, "responder"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Confirm the change through the roster read.
    let roster = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/members"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(roster.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let row = json
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["user_id"] == member.to_string())
        .unwrap();
    assert_eq!(row["role"], "responder");
}

#[tokio::test]
async fn set_member_role_is_forbidden_for_a_non_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Responder);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, member, "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn set_member_role_on_an_unknown_member_is_not_found() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, Uuid::new_v4(), "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn set_member_role_cannot_target_the_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let manager = Uuid::nil();
    ctx.teams.seed_member(team_id, manager, Role::Manager);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, manager, "responder"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn set_member_role_rejects_an_invalid_role() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let member = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, member, Role::Observer);

    let response = ctx
        .app
        .oneshot(set_role_request(team_id, member, "manager"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
