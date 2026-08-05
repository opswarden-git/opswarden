// Sign-in throttling. Split from auth.rs to keep both files inside the
// 500-line source hygiene ceiling; included rather than declared as a module,
// following tests/incidents.rs.

#[tokio::test]
async fn repeated_sign_in_attempts_from_one_address_are_rate_limited() {
    let ctx = common::test_context_with_address_rate_limit(3, 300);
    let payload = json!({ "email": "victim@test.com", "password": "wrong-guess" });

    let attempt = |app: axum::Router| {
        let body = payload.to_string();
        async move {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/sign-in")
                    .extension(client_addr())
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };

    // The budget is spent on failed guesses, which still answer 401.
    for _ in 0..3 {
        let response = attempt(ctx.app.clone()).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // The next guess never reaches the handler.
    let blocked = attempt(ctx.app.clone()).await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = blocked
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("a throttled response must tell the caller when to retry");
    assert!(retry_after >= 1);
}

#[tokio::test]
async fn one_address_cannot_lock_out_another() {
    let ctx = common::test_context_with_address_rate_limit(2, 300);
    let payload = json!({ "email": "victim@test.com", "password": "wrong-guess" });

    let attempt = |app: axum::Router, peer: &'static str| {
        let body = payload.to_string();
        async move {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/sign-in")
                    .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()))
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };

    for _ in 0..2 {
        attempt(ctx.app.clone(), "198.51.100.10:1111").await;
    }
    assert_eq!(
        attempt(ctx.app.clone(), "198.51.100.10:1111")
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );

    // A different caller keeps its own untouched budget.
    assert_eq!(
        attempt(ctx.app.clone(), "198.51.100.20:2222")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn exhausting_one_account_never_locks_out_another() {
    // The address-keyed ceiling is useless behind a proxy that forwards no
    // client address: Compose's Next client is one, so every visitor arrives
    // as the same peer. The limit that has to hold is the per-account one.
    let ctx = common::test_context_with_auth_rate_limit(2, 300);

    let attempt = |app: axum::Router, email: &'static str| async move {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .extension(client_addr())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "wrong-guess" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
    };

    for _ in 0..2 {
        let response = attempt(ctx.app.clone(), "existing@test.com").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    assert_eq!(
        attempt(ctx.app.clone(), "existing@test.com").await.status(),
        StatusCode::TOO_MANY_REQUESTS,
    );

    // Same peer address, different account: untouched.
    assert_eq!(
        attempt(ctx.app.clone(), "unknown@test.com").await.status(),
        StatusCode::UNAUTHORIZED,
    );
}

#[tokio::test]
async fn account_budget_ignores_address_and_casing() {
    let ctx = common::test_context_with_auth_rate_limit(2, 300);

    let attempt = |app: axum::Router, email: &'static str, peer: &'static str| async move {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "wrong-guess" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
    };

    attempt(ctx.app.clone(), "existing@test.com", "198.51.100.1:1111").await;
    // A different address and different casing must not buy a fresh budget.
    attempt(ctx.app.clone(), "EXISTING@test.com", "198.51.100.2:2222").await;
    assert_eq!(
        attempt(ctx.app.clone(), " Existing@Test.com ", "198.51.100.3:3333")
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS,
    );
}

#[tokio::test]
async fn successful_sign_ins_never_spend_the_account_budget() {
    // A limiter that meters attempts also meters the legitimate ones. Only
    // failures are evidence of guessing, so a shift handover — or this suite —
    // must be able to sign in far past the budget.
    let ctx = common::test_context_with_auth_rate_limit(2, 300);

    for _ in 0..10 {
        let response = ctx
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/sign-in")
                    .extension(client_addr())
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({ "email": "existing@test.com", "password": "correct_password" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
