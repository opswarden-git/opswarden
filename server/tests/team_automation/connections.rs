#[tokio::test]
async fn manager_configures_and_lists_team_connection_without_secret_material() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_github(&ctx, team_id).await;
    assert_eq!(configured["team_id"], team_id.to_string());
    assert_eq!(configured["service"], "github");
    assert_eq!(configured["secret_configured"], true);
    assert_eq!(configured["token_configured"], true);
    assert_eq!(
        configured["webhook_path"],
        format!("/webhooks/github/{}", configured["id"].as_str().unwrap())
    );
    let serialized = configured.to_string();
    assert!(!serialized.contains(SIGNING_SECRET));
    assert!(!serialized.contains(PERSONAL_TOKEN));
    assert!(!serialized.contains("ciphertext"));
    assert!(!serialized.contains("nonce"));

    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let listed = json_body(response).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let serialized = listed.to_string();
    assert!(!serialized.contains(SIGNING_SECRET));
    assert!(!serialized.contains(PERSONAL_TOKEN));
}

#[tokio::test]
async fn catalog_service_route_configures_known_services_and_rejects_unknown_ones() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/github"),
            Some(serde_json::json!({
                "webhook_signing_secret": SIGNING_SECRET
            })),
        ))
        .await
        .unwrap();
    assert_eq!(configured.status(), StatusCode::OK);
    assert_eq!(json_body(configured).await["service"], "github");

    let gitlab = configure_gitlab(&ctx, team_id).await;
    assert_eq!(gitlab["service"], "gitlab");
    assert_eq!(gitlab["secret_configured"], true);
    assert_eq!(
        gitlab["webhook_path"],
        format!("/webhooks/gitlab/{}", gitlab["id"].as_str().unwrap())
    );
    assert!(!gitlab.to_string().contains(GITLAB_TOKEN));

    let generic = configure_generic(&ctx, team_id).await;
    assert_eq!(generic["service"], "generic");
    assert_eq!(generic["secret_configured"], true);
    assert_eq!(
        generic["webhook_path"],
        format!("/webhooks/generic/{}", generic["id"].as_str().unwrap())
    );
    assert!(!generic.to_string().contains(GENERIC_TOKEN));

    let unknown = ctx
        .app
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/unknown"),
            Some(serde_json::json!({})),
        ))
        .await
        .unwrap();
    assert!(!unknown.status().is_success());
    assert_eq!(
        ctx.service_connections
            .list_connections_for_team(team_id)
            .await
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn manager_configures_and_tests_http_without_exposing_the_endpoint() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_http(&ctx, team_id).await;
    assert_eq!(configured["service"], "http");
    assert_eq!(configured["endpoint_configured"], true);
    assert!(!configured.to_string().contains(HTTP_ENDPOINT));
    let connection_id = configured["id"].as_str().unwrap();
    assert_eq!(
        ctx.connection_credentials
            .reveal_credential(
                Uuid::parse_str(connection_id).unwrap(),
                opswarden_server::domain::automation_config::CredentialKind::EndpointUrl,
            )
            .await
            .unwrap()
            .as_deref(),
        Some(HTTP_ENDPOINT)
    );

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::NO_CONTENT);
    assert_eq!(ctx.notifier.calls().len(), 1);
    assert_eq!(ctx.notifier.calls()[0].1, "OpsWarden connection test");
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, Uuid::parse_str(connection_id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.verified_at.is_some());
    assert!(persisted.last_delivery_at.is_none());
    assert_eq!(persisted.last_error_code, None);
}

#[tokio::test]
async fn manager_configures_and_tests_email_without_exposing_the_smtp_password() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let configured = configure_email(&ctx, team_id).await;
    assert_eq!(configured["service"], "email");
    assert!(!configured.to_string().contains(SMTP_PASSWORD));
    let connection_id = Uuid::parse_str(configured["id"].as_str().unwrap()).unwrap();
    for (kind, expected) in [
        (CredentialKind::SmtpHost, "smtp.example.com"),
        (CredentialKind::SmtpPort, "587"),
        (CredentialKind::SmtpUsername, "opswarden"),
        (CredentialKind::SmtpPassword, SMTP_PASSWORD),
        (CredentialKind::FromAddress, "alerts@example.com"),
    ] {
        assert_eq!(
            ctx.connection_credentials
                .reveal_credential(connection_id, kind)
                .await
                .unwrap()
                .as_deref(),
            Some(expected)
        );
    }

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::NO_CONTENT);
    // The probe opens an SMTP session; it must never deliver a message.
    let validated = ctx.email_sender.validated();
    assert_eq!(validated.len(), 1);
    assert_eq!(validated[0].host, "smtp.example.com");
    assert_eq!(validated[0].port, 587);
    assert_eq!(validated[0].from, "alerts@example.com");
    assert!(ctx.email_sender.sent().is_empty());

    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, connection_id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.verified_at.is_some());
    assert_eq!(persisted.last_error_code, None);
}

#[tokio::test]
async fn failed_email_test_records_the_transport_error_code() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let configured = configure_email(&ctx, team_id).await;
    let connection_id = Uuid::parse_str(configured["id"].as_str().unwrap()).unwrap();
    ctx.email_sender
        .fail_with(|| DomainError::EmailTransportError);

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::BAD_GATEWAY);
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, connection_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.last_error_code.as_deref(),
        Some("email_transport_error")
    );
}

#[tokio::test]
async fn creating_an_email_connection_requires_every_credential() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    let partial = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/email"),
            Some(json!({"smtp_host": "smtp.example.com", "smtp_port": "587"})),
        ))
        .await
        .unwrap();
    assert_eq!(partial.status(), StatusCode::BAD_REQUEST);
    assert!(ctx
        .service_connections
        .find_connection_by_service(team_id, "email")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn an_email_connection_rejects_a_malformed_port_or_sender() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);

    for payload in [
        json!({
            "smtp_host": "smtp.example.com",
            "smtp_port": "not-a-port",
            "smtp_username": "opswarden",
            "smtp_password": SMTP_PASSWORD,
            "from_address": "alerts@example.com"
        }),
        json!({
            "smtp_host": "smtp.example.com",
            "smtp_port": "587",
            "smtp_username": "opswarden",
            "smtp_password": SMTP_PASSWORD,
            "from_address": "not-an-address"
        }),
    ] {
        let response = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/email"),
                Some(payload),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn a_responder_cannot_configure_an_email_connection() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Responder);

    let response = ctx
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/teams/{team_id}/service-connections/by-service/email"),
            Some(email_payload()),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn failed_http_test_records_only_a_safe_error_code() {
    let ctx = test_context();
    let team_id = Uuid::new_v4();
    ctx.teams.seed_member(team_id, REQUESTER, Role::Manager);
    let configured = configure_http(&ctx, team_id).await;
    let connection_id = configured["id"].as_str().unwrap();
    ctx.notifier.fail_requests();

    let tested = ctx
        .app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/teams/{team_id}/service-connections/{connection_id}/test"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(tested.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(json_body(tested).await["code"], "reaction_http_5xx");
    let persisted = ctx
        .service_connections
        .find_connection_for_team(team_id, Uuid::parse_str(connection_id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.last_error_code.as_deref(),
        Some("reaction_http_5xx")
    );
    assert!(!format!("{persisted:?}").contains(HTTP_ENDPOINT));
}

#[tokio::test]
async fn only_manager_can_read_connections_or_runs() {
    for role in [Role::Responder, Role::Observer] {
        let ctx = test_context();
        let team_id = Uuid::new_v4();
        ctx.teams.seed_member(team_id, REQUESTER, role);

        for suffix in ["service-connections", "automation-rules", "automation-runs"] {
            let response = ctx
                .app
                .clone()
                .oneshot(request(
                    "GET",
                    &format!("/api/teams/{team_id}/{suffix}"),
                    None,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert_eq!(json_body(response).await["code"], "not_manager");
        }

        let configure = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/github"),
                Some(json!({"webhook_signing_secret": SIGNING_SECRET})),
            ))
            .await
            .unwrap();
        assert_eq!(configure.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure).await["code"], "not_manager");

        let configure_generic = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/generic"),
                Some(json!({"webhook_signing_secret": GENERIC_TOKEN})),
            ))
            .await
            .unwrap();
        assert_eq!(configure_generic.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure_generic).await["code"], "not_manager");

        let configure_http = ctx
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/teams/{team_id}/service-connections/by-service/http"),
                Some(json!({"endpoint_url": HTTP_ENDPOINT})),
            ))
            .await
            .unwrap();
        assert_eq!(configure_http.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(configure_http).await["code"], "not_manager");

        let test_http = ctx
            .app
            .clone()
            .oneshot(request(
                "POST",
                &format!(
                    "/api/teams/{team_id}/service-connections/{}/test",
                    Uuid::new_v4()
                ),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(test_http.status(), StatusCode::FORBIDDEN);
        assert_eq!(json_body(test_http).await["code"], "not_manager");
        assert!(ctx.connection_credentials.raw_values().is_empty());
    }

    let ctx = test_context();
    let team_id = Uuid::new_v4();
    let response = ctx
        .app
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_id}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(response).await["code"], "forbidden");
}

#[tokio::test]
async fn manager_of_team_a_cannot_read_or_delete_team_b_connection() {
    let ctx = test_context();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();
    ctx.teams.seed_member(team_a, REQUESTER, Role::Manager);
    let owner_b = Uuid::new_v4();
    let connection_b = ServiceConnection::new(team_b, "github", owner_b).unwrap();
    ctx.service_connections
        .insert_connection(&connection_b)
        .await
        .unwrap();

    let read = ctx
        .app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/teams/{team_b}/service-connections"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::FORBIDDEN);

    let delete = ctx
        .app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!(
                "/api/teams/{team_b}/service-connections/{}",
                connection_b.id
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::FORBIDDEN);
    assert!(ctx
        .service_connections
        .find_connection_for_team(team_b, connection_b.id)
        .await
        .unwrap()
        .is_some());
}
