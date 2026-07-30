#[tokio::test]
async fn github_oauth_flow_stores_tokens_without_returning_them() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let start = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/api/teams/{team_id}/service-connections/by-service/github/oauth/start?locale=fr"
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::OK);
    let set_cookie = start
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    let cookie = set_cookie.split(';').next().unwrap().to_string();
    let started = json_body(start).await;
    let authorization_url = started["authorization_url"].as_str().unwrap();
    assert!(authorization_url.starts_with("https://github.test/"));
    assert!(authorization_url.contains("code_challenge_method=S256"));
    assert!(!authorization_url.contains(OAUTH_ACCESS));
    assert!(!authorization_url.contains(OAUTH_REFRESH));
    let returned_state = authorization_url
        .split("state=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/service-oauth/github/callback?code=provider-code&state={returned_state}"
                ))
                .header("Cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(callback.status().is_redirection());
    let location = callback
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains(&format!("/fr/teams/{team_id}/automations?view=connections")));
    assert!(!location.contains(OAUTH_ACCESS));
    assert!(!location.contains(OAUTH_REFRESH));

    let connection = ctx
        .service_connections
        .find_connection_by_service(team_id, "github")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthAccessToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_ACCESS)
    );
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_REFRESH)
    );
    let exchanged = ctx.service_oauth.exchanges();
    assert_eq!(exchanged.len(), 1);
    assert_eq!(exchanged[0].0, "provider-code");
    assert!((43..=128).contains(&exchanged[0].1.len()));

    let listed = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    let listed = json_body(listed).await;
    assert_eq!(listed[0]["oauth_configured"], true);
    assert_eq!(listed[0]["oauth_refresh_configured"], true);
    let serialized = listed.to_string();
    for secret in [OAUTH_ACCESS, OAUTH_REFRESH, "provider-code"] {
        assert!(!serialized.contains(secret));
    }
}

#[tokio::test]
async fn github_oauth_refresh_rotates_both_encrypted_credentials() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let connection = ServiceConnection::new(team_id, "github", REQUESTER).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::OAuthAccessToken,
            OAUTH_ACCESS,
        )
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::OAuthRefreshToken,
            OAUTH_REFRESH,
        )
        .await
        .unwrap();

    let refreshed = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!(
                "/api/teams/{team_id}/service-connections/{}/oauth/refresh",
                connection.id
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(refreshed.status(), StatusCode::OK);
    let response = json_body(refreshed).await;
    assert_eq!(response["oauth_configured"], true);
    assert_eq!(response["oauth_refresh_configured"], true);
    let serialized = response.to_string();
    assert!(!serialized.contains(OAUTH_ACCESS_ROTATED));
    assert!(!serialized.contains(OAUTH_REFRESH_ROTATED));
    assert_eq!(ctx.service_oauth.refreshes(), vec![OAUTH_REFRESH]);
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthAccessToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_ACCESS_ROTATED)
    );
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
            .await
            .unwrap()
            .as_deref(),
        Some(OAUTH_REFRESH_ROTATED)
    );
}

#[tokio::test]
async fn github_oauth_callback_rejects_mismatched_state_and_tampered_context() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let started = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/by-service/github/oauth/start"),
            None,
        ))
        .await
        .unwrap();
    let cookie = started
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let authorization_url = json_body(started).await["authorization_url"]
        .as_str()
        .unwrap()
        .to_string();
    let valid_state = authorization_url
        .split("state=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/service-oauth/github/callback?code=provider-code&state=attacker")
                .header("Cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!callback.status().is_success());

    let mut tampered_cookie = cookie.into_bytes();
    let last = tampered_cookie.len() - 1;
    tampered_cookie[last] = if tampered_cookie[last] == b'a' {
        b'b'
    } else {
        b'a'
    };
    let callback = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/service-oauth/github/callback?code=provider-code&state={valid_state}"
                ))
                .header("Cookie", String::from_utf8(tampered_cookie).unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!callback.status().is_success());
    assert!(ctx.service_oauth.exchanges().is_empty());
    assert!(ctx.connection_credentials.raw_values().is_empty());
}
