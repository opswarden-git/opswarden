#[tokio::test]
async fn leave_team_removes_member_when_not_manager() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();

    ctx.teams.seed_member(team_id, requester, Role::Responder);
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events
        .register(requester, HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/leave"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn manager_can_delete_team() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let requester = Uuid::nil();

    ctx.teams.seed_member(team_id, requester, Role::Manager);
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events
        .register(requester, HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn list_teams_returns_the_users_teams_with_roles() {
    let ctx = test_context();
    let team = Team::new("SRE Core").unwrap();
    let observed_team = Team::new("Read only").unwrap();
    ctx.teams.seed_team(team.clone());
    ctx.teams.seed_team(observed_team.clone());
    ctx.teams.seed_member(team.id, Uuid::nil(), Role::Manager);
    ctx.teams
        .seed_member(observed_team.id, Uuid::nil(), Role::Observer);

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/teams")
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
    let teams = json.as_array().unwrap();
    assert_eq!(teams.len(), 2);
    let manager_team = teams.iter().find(|row| row["name"] == "SRE Core").unwrap();
    assert_eq!(manager_team["role"], "manager");
    assert!(manager_team.get("invitation_code").is_none());
    assert!(manager_team["created_at"].is_string());
    assert_eq!(manager_team["member_count"], 1);
    assert_eq!(manager_team["active_incident_count"], 0);
    assert_eq!(manager_team["active_release_count"], 0);
    assert_eq!(manager_team["blocked_release_count"], 0);

    let observer_team = teams.iter().find(|row| row["name"] == "Read only").unwrap();
    assert_eq!(observer_team["role"], "observer");
    assert!(observer_team.get("invitation_code").is_none());
}

#[tokio::test]
async fn invitation_code_uses_a_manager_only_endpoint() {
    let ctx = test_context();
    let team = Team::new("SRE Core").unwrap();
    ctx.teams.seed_team(team.clone());
    ctx.teams.seed_member(team.id, Uuid::nil(), Role::Manager);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{}/invitation", team.id))
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
    assert_eq!(json["invitation_code"], team.invitation_code.as_str());
}

// The integration harness authenticates every request as `Uuid::nil()`, so the
// Manager (or the joining user) is seeded as nil and the target is a separate id.

#[tokio::test]
async fn manager_kicks_a_member_over_http() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let observer = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, observer, Role::Observer);
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events
        .register(observer, HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/members/{observer}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(ctx.teams.role_for(team_id, observer), None);
    while rx.try_recv().is_ok() {}
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn non_manager_cannot_kick_over_http() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let target = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Responder);
    ctx.teams.seed_member(team_id, target, Role::Observer);

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/members/{target}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(ctx.teams.role_for(team_id, target), Some(Role::Observer));
}

#[tokio::test]
async fn manager_permanently_bans_a_member_and_drops_membership() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let observer = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams.seed_member(team_id, observer, Role::Observer);
    let (tx, mut rx) = mpsc::channel(8);
    ctx.events
        .register(observer, HashSet::from([team_id]), tx);
    while rx.try_recv().is_ok() {}

    let payload = serde_json::json!({ "user_id": observer, "kind": "permanent" });
    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{team_id}/bans"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    // Membership is dropped by the ban.
    assert_eq!(ctx.teams.role_for(team_id, observer), None);
    while rx.try_recv().is_ok() {}
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn a_banned_user_cannot_join() {
    let ctx = test_context();
    let team = Team::new("Locked").unwrap();
    let code = team.invitation_code.as_str().to_string();
    ctx.teams.seed_team(team.clone());
    // The joining user is the authenticated nil user; ban them.
    ctx.teams.seed_ban(TeamBan::permanent(
        team.id,
        Uuid::nil(),
        Uuid::new_v4(),
        None,
    ));

    let payload = serde_json::json!({ "invitation_code": code });
    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams/join")
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(ctx.teams.role_for(team.id, Uuid::nil()), None);
}

#[tokio::test]
async fn the_ban_list_is_manager_only() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let banned = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams
        .seed_ban(TeamBan::permanent(team_id, banned, Uuid::nil(), None));

    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team_id}/bans"))
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
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["user"]["user_id"], banned.to_string());
    assert!(json[0]["user"]["email"].as_str().unwrap().contains('@'));
    assert_eq!(json[0]["kind"], "permanent");
    assert_eq!(json[0]["active"], true);

    // A non-manager is forbidden.
    let ctx2 = test_context();
    let team2 = Uuid::new_v4();
    ctx2.teams.seed_member(team2, Uuid::nil(), Role::Observer);
    let forbidden = ctx2
        .app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/teams/{team2}/bans"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn manager_can_lift_a_ban() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let banned = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    ctx.teams
        .seed_ban(TeamBan::permanent(team_id, banned, Uuid::nil(), None));

    let response = ctx
        .app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/bans/{banned}"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(ctx.teams.list_bans(team_id).await.unwrap().is_empty());
}
