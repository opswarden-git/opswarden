fn signature(secret: &str, body: &str) -> String {
    format!(
        "sha256={}",
        hex::encode(hmac_sha256(secret.as_bytes(), body.as_bytes()))
    )
}

fn webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    secret: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/github/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-GitHub-Delivery", delivery_id)
        .header("X-GitHub-Event", event)
        .header("X-Hub-Signature-256", signature(secret, body))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn gitlab_webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    token: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/gitlab/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-Gitlab-Event-UUID", delivery_id)
        .header("X-Gitlab-Event", event)
        .header("X-Gitlab-Token", token)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn generic_webhook_request(
    connection_id: Uuid,
    delivery_id: &str,
    event: &str,
    token: &str,
    body: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/webhooks/generic/{connection_id}"))
        .header("Content-Type", "application/json")
        .header("X-OpsWarden-Delivery", delivery_id)
        .header("X-OpsWarden-Event", event)
        .header("X-OpsWarden-Token", token)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_automation(
    ctx: &common::TestContext,
    team_id: Uuid,
    secret: &str,
    trigger_config: Value,
    reaction_kind: &str,
) -> (ServiceConnection, AutomationRule) {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(connection.id, CredentialKind::WebhookSigningSecret, secret)
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "GitHub CI failed",
        connection.id,
        "ci_failed",
        trigger_config,
        reaction_kind,
        None,
        json!({"severity": "critical"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (connection, rule)
}

async fn seed_http_automation(
    ctx: &common::TestContext,
    team_id: Uuid,
    secret: &str,
) -> (ServiceConnection, ServiceConnection, AutomationRule) {
    let user_id = Uuid::new_v4();
    let github = ServiceConnection::new(team_id, "github", user_id).unwrap();
    let http = ServiceConnection::new(team_id, "http", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&github)
        .await
        .unwrap();
    ctx.service_connections
        .insert_connection(&http)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(github.id, CredentialKind::WebhookSigningSecret, secret)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            http.id,
            CredentialKind::EndpointUrl,
            "https://hooks.example.com/opswarden-secret",
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "GitHub CI failed to HTTP",
        github.id,
        "ci_failed",
        json!({}),
        "http_notify",
        Some(http.id),
        json!({"message": "Alert: {{workflow}} / {{repository}}"}),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (github, http, rule)
}

async fn seed_email_automation(
    ctx: &common::TestContext,
    team_id: Uuid,
    reaction_config: Value,
) -> (ServiceConnection, ServiceConnection, AutomationRule) {
    let user_id = Uuid::new_v4();
    let github = ServiceConnection::new(team_id, "github", user_id).unwrap();
    let email = ServiceConnection::new(team_id, "email", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&github)
        .await
        .unwrap();
    ctx.service_connections
        .insert_connection(&email)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(github.id, CredentialKind::WebhookSigningSecret, SECRET_A)
        .await
        .unwrap();
    for (kind, value) in [
        (CredentialKind::SmtpHost, "smtp.example.com"),
        (CredentialKind::SmtpPort, "587"),
        (CredentialKind::SmtpUsername, "opswarden"),
        (CredentialKind::SmtpPassword, "smtp-password"),
        (CredentialKind::FromAddress, "alerts@example.com"),
    ] {
        ctx.connection_credentials
            .store_credential(email.id, kind, value)
            .await
            .unwrap();
    }
    let mut rule = AutomationRule::new(
        team_id,
        "GitHub CI failed to email",
        github.id,
        "ci_failed",
        json!({}),
        "email_notify",
        Some(email.id),
        reaction_config,
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (github, email, rule)
}
