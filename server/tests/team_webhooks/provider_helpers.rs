async fn seed_github_action(
    ctx: &common::TestContext,
    team_id: Uuid,
    trigger_kind: &str,
    trigger_config: Value,
    reaction_config: Value,
) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            SECRET_A,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitHub {trigger_kind}"),
        connection.id,
        trigger_kind,
        trigger_config,
        "create_incident",
        None,
        reaction_config,
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}

async fn seed_github_reaction(
    ctx: &common::TestContext,
    team_id: Uuid,
    reaction_kind: &str,
    reaction_config: Value,
) -> (ServiceConnection, AutomationRule) {
    let actor = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "github", actor).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            SECRET_A,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitHub to {reaction_kind}"),
        connection.id,
        "ci_failed",
        json!({}),
        reaction_kind,
        None,
        reaction_config,
        actor,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    (connection, rule)
}

async fn seed_gitlab_action(
    ctx: &common::TestContext,
    team_id: Uuid,
    trigger_kind: &str,
    trigger_config: Value,
    reaction_config: Value,
) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "gitlab", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            GITLAB_TOKEN,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        format!("GitLab {trigger_kind}"),
        connection.id,
        trigger_kind,
        trigger_config,
        "create_incident",
        None,
        reaction_config,
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}

async fn seed_generic_action(ctx: &common::TestContext, team_id: Uuid) -> ServiceConnection {
    let user_id = Uuid::new_v4();
    let connection = ServiceConnection::new(team_id, "generic", user_id).unwrap();
    ctx.service_connections
        .insert_connection(&connection)
        .await
        .unwrap();
    ctx.connection_credentials
        .store_credential(
            connection.id,
            CredentialKind::WebhookSigningSecret,
            GENERIC_TOKEN,
        )
        .await
        .unwrap();
    let mut rule = AutomationRule::new(
        team_id,
        "Generic deployment failure",
        connection.id,
        "generic_event",
        json!({
            "event_type": "deployment_failed",
            "source": "jury",
            "severity": "critical"
        }),
        "create_incident",
        None,
        json!({
            "severity": "critical",
            "title": "{{source}}: {{title}} ({{external_id}})"
        }),
        user_id,
    )
    .unwrap();
    rule.set_enabled(true);
    ctx.automation_rules.insert_rule(&rule).await.unwrap();
    connection
}
