#[tokio::test]
async fn manager_can_replace_read_and_delete_the_team_image() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    let png = b"\x89PNG\r\n\x1a\nteam-avatar";
    let payload = serde_json::json!({
        "media_type": "image/png",
        "data_base64": base64::engine::general_purpose::STANDARD.encode(png),
    });

    let updated = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);

    let loaded = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(loaded.status(), StatusCode::OK);
    assert_eq!(loaded.headers()["content-type"], "image/png");
    assert_eq!(loaded.headers()["cache-control"], "private, max-age=31536000, immutable");
    assert_eq!(loaded.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        axum::body::to_bytes(loaded.into_body(), usize::MAX)
            .await
            .unwrap(),
        png.as_slice()
    );

    let deleted = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let missing = ctx
        .app
        .oneshot(
            Request::builder()
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn team_image_rejects_non_managers_and_invalid_content() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Observer);
    let valid = serde_json::json!({
        "media_type": "image/png",
        "data_base64": base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nvalid"),
    });
    let forbidden = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(valid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    ctx.teams.seed_member(team_id, Uuid::nil(), Role::Manager);
    let invalid = serde_json::json!({
        "media_type": "image/png",
        "data_base64": base64::engine::general_purpose::STANDARD.encode(b"not a png"),
    });
    let rejected = ctx
        .app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/teams/{team_id}/image"))
                .header("Authorization", "Bearer mock_jwt_token")
                .header("Content-Type", "application/json")
                .body(Body::from(invalid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
}
