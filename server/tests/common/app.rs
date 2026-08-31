pub fn test_context() -> TestContext {
    // Suites replay sign-in and sign-up many times from one synthetic address;
    // a production-sized budget would make them flaky for a reason unrelated to
    // what they assert. `test_context_with_auth_rate_limit` covers the limiter.
    build_context(10_000, 10_000, 300)
}

/// Build a context whose auth routes carry a deliberately small budget.
///
/// The address ceiling stays wide so a suite can exercise the per-account limit
/// without the coarse one firing first and hiding which rule actually held.
#[allow(dead_code)]
pub fn test_context_with_auth_rate_limit(attempts: u32, window_seconds: u64) -> TestContext {
    build_context(10_000, attempts, window_seconds)
}

/// Build a context whose coarse address ceiling is the one under test.
#[allow(dead_code)]
pub fn test_context_with_address_rate_limit(attempts: u32, window_seconds: u64) -> TestContext {
    build_context(attempts, 10_000, window_seconds)
}
fn build_context(
    address_attempts: u32,
    account_attempts: u32,
    auth_window_seconds: u64,
) -> TestContext {
    let users = Arc::new(DummyUserRepo::default());
    let teams = Arc::new(DummyTeamRepo::default());
    let incidents = Arc::new(DummyIncidentRepo::default());
    let timeline = Arc::new(DummyTimelineRepo::default());
    let private_messages = Arc::new(DummyPrivateMessageRepo::default());
    let releases = Arc::new(DummyReleaseRepo::new(incidents.clone()));
    let revoked_tokens = Arc::new(DummyTokenRevocationRepo::default());
    let events = Arc::new(WsHub::new());
    let service_connections = Arc::new(DummyServiceConnectionRepo::default());
    let connection_credentials = Arc::new(DummyConnectionCredentialVault::new(
        &service_connections,
    ));
    let service_oauth = Arc::new(DummyServiceOAuthClient::default());
    let automation_rules = Arc::new(DummyAutomationRuleRepo::default());
    let webhook_deliveries = Arc::new(DummyWebhookDeliveryRepo::default());
    let automation_runs = Arc::new(DummyAutomationRunRepo::default());
    let notifier = Arc::new(DummyNotifier::default());
    let email_sender = Arc::new(DummyEmailSender::default());
    let alertmanager_metrics =
        Arc::new(opswarden_server::adapters::metrics::AlertmanagerWebhookMetrics::default());
    let webhook_ingress = Arc::new(
        opswarden_server::app::automation::IngestTeamWebhookUseCase::new(
            opswarden_server::app::automation::TeamWebhookDependencies {
                connections: service_connections.clone(),
                credentials: connection_credentials.clone(),
                verifier: Arc::new(HmacSha256Verifier),
                parser: Arc::new(
                    opswarden_server::adapters::webhook::CompositeWebhookParser::new(),
                ),
                deliveries: webhook_deliveries.clone(),
                rules: automation_rules.clone(),
                runs: automation_runs.clone(),
                incidents: incidents.clone(),
                releases: releases.clone(),
                notifier: notifier.clone(),
                events: events.clone(),
                email_sender: email_sender.clone(),
            },
        ),
    );
    let mut config = Config::for_test();
    // HTTP tests inject ConnectInfo explicitly and must not inherit a developer
    // machine's reverse-proxy trust setting.
    config.trusted_proxy_hops = 0;
    config.auth_rate_limit_attempts = address_attempts;
    config.auth_rate_limit_per_account = account_attempts;
    config.auth_rate_limit_window_seconds = auth_window_seconds;
    let app = build_app(AppState {
        users: users.clone(),
        teams: teams.clone(),
        incidents: incidents.clone(),
        timeline: timeline.clone(),
        private_messages: private_messages.clone(),
        releases: releases.clone(),
        hasher: Arc::new(DummyHasher),
        tokens: Arc::new(DummyTokenService),
        oauth: Arc::new(DummyOAuthClient),
        github_auth_oauth: Arc::new(DummyGithubAuthOAuthClient),
        service_oauth: service_oauth.clone(),
        token_revocations: revoked_tokens.clone(),
        events: events.clone(),
        clock: Arc::new(DummyClock),
        webhook_ingress,
        alertmanager_metrics: alertmanager_metrics.clone(),
        service_connections: service_connections.clone(),
        connection_credentials: connection_credentials.clone(),
        automation_rules: automation_rules.clone(),
        webhook_deliveries: webhook_deliveries.clone(),
        automation_runs: automation_runs.clone(),
        notifier: notifier.clone(),
        email_sender: email_sender.clone(),
        gifs: Arc::new(DummyGifSearch),
        auth_rate_limiter: Arc::new(
            opswarden_server::adapters::rate_limit::RateLimiter::new(
                config.auth_rate_limit_attempts,
                config.auth_rate_limit_window_seconds,
            ),
        ),
        account_rate_limiter: Arc::new(
            opswarden_server::adapters::rate_limit::RateLimiter::new(
                config.auth_rate_limit_per_account,
                config.auth_rate_limit_window_seconds,
            ),
        ),
        config,
    });
    TestContext {
        app,
        users,
        teams,
        incidents,
        timeline,
        private_messages,
        releases,
        revoked_tokens,
        events,
        service_connections,
        connection_credentials,
        service_oauth,
        automation_rules,
        webhook_deliveries,
        automation_runs,
        notifier,
        email_sender,
    }
}

#[allow(dead_code)]
pub fn test_app() -> axum::Router {
    test_context().app
}
