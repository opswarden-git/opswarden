#[derive(Default)]
pub struct DummyServiceConnectionRepo {
    connections: Mutex<HashMap<Uuid, ServiceConnection>>,
}

#[async_trait]
impl ServiceConnectionRepo for DummyServiceConnectionRepo {
    async fn insert_connection(&self, connection: &ServiceConnection) -> Result<(), DomainError> {
        self.connections
            .lock()
            .unwrap()
            .insert(connection.id, connection.clone());
        Ok(())
    }

    async fn find_connection_by_id(
        &self,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        Ok(self
            .connections
            .lock()
            .unwrap()
            .get(&connection_id)
            .cloned())
    }

    async fn find_connection_for_team(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        Ok(self
            .connections
            .lock()
            .unwrap()
            .get(&connection_id)
            .filter(|connection| connection.team_id == team_id)
            .cloned())
    }

    async fn find_connection_by_service(
        &self,
        team_id: Uuid,
        service: &str,
    ) -> Result<Option<ServiceConnection>, DomainError> {
        Ok(self
            .connections
            .lock()
            .unwrap()
            .values()
            .find(|connection| connection.team_id == team_id && connection.service == service)
            .cloned())
    }

    async fn list_connections_for_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<ServiceConnection>, DomainError> {
        let mut connections: Vec<_> = self
            .connections
            .lock()
            .unwrap()
            .values()
            .filter(|connection| connection.team_id == team_id)
            .cloned()
            .collect();
        connections.sort_by(|left, right| left.service.cmp(&right.service));
        Ok(connections)
    }

    async fn record_delivery_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut connections = self.connections.lock().unwrap();
        let connection = connections
            .get_mut(&connection_id)
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        let now = Utc::now();
        connection.verified_at.get_or_insert(now);
        connection.last_delivery_at = Some(now);
        connection.last_error_code = error_code.map(str::to_string);
        connection.updated_at = now;
        Ok(())
    }

    async fn record_reaction_result(
        &self,
        connection_id: Uuid,
        error_code: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut connections = self.connections.lock().unwrap();
        let connection = connections
            .get_mut(&connection_id)
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if error_code.is_none() && connection.verified_at.is_none() {
            connection.verified_at = Some(Utc::now());
        }
        connection.last_error_code = error_code.map(str::to_string);
        connection.updated_at = Utc::now();
        Ok(())
    }

    async fn reset_connection_health(&self, connection_id: Uuid) -> Result<(), DomainError> {
        let mut connections = self.connections.lock().unwrap();
        let connection = connections
            .get_mut(&connection_id)
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        connection.verified_at = None;
        connection.last_error_code = None;
        connection.updated_at = Utc::now();
        Ok(())
    }

    async fn delete_connection(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut connections = self.connections.lock().unwrap();
        let belongs_to_team = connections
            .get(&connection_id)
            .is_some_and(|connection| connection.team_id == team_id);
        if belongs_to_team {
            connections.remove(&connection_id);
        }
        Ok(belongs_to_team)
    }
}

#[derive(Default)]
pub struct DummyConnectionCredentialVault {
    credentials: Mutex<HashMap<(Uuid, CredentialKind), String>>,
}

#[allow(dead_code)]
impl DummyConnectionCredentialVault {
    pub fn raw_values(&self) -> Vec<String> {
        self.credentials.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl ConnectionCredentialVault for DummyConnectionCredentialVault {
    async fn store_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
        secret: &str,
    ) -> Result<(), DomainError> {
        if secret.trim().is_empty() {
            return Err(DomainError::InvalidServiceSecret);
        }
        self.credentials
            .lock()
            .unwrap()
            .insert((connection_id, kind), secret.to_string());
        Ok(())
    }

    async fn reveal_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<Option<String>, DomainError> {
        Ok(self
            .credentials
            .lock()
            .unwrap()
            .get(&(connection_id, kind))
            .cloned())
    }

    async fn delete_credential(
        &self,
        connection_id: Uuid,
        kind: CredentialKind,
    ) -> Result<(), DomainError> {
        self.credentials
            .lock()
            .unwrap()
            .remove(&(connection_id, kind));
        Ok(())
    }

    async fn configured_credential_kinds(
        &self,
        connection_id: Uuid,
    ) -> Result<Vec<CredentialKind>, DomainError> {
        let mut kinds: Vec<_> = self
            .credentials
            .lock()
            .unwrap()
            .keys()
            .filter_map(|(id, kind)| (*id == connection_id).then_some(*kind))
            .collect();
        kinds.sort_by_key(ToString::to_string);
        Ok(kinds)
    }
}

#[derive(Default)]
pub struct DummyAutomationRuleRepo {
    rules: Mutex<HashMap<Uuid, AutomationRule>>,
}

#[async_trait]
impl AutomationRuleRepo for DummyAutomationRuleRepo {
    async fn insert_rule(&self, rule: &AutomationRule) -> Result<(), DomainError> {
        self.rules.lock().unwrap().insert(rule.id, rule.clone());
        Ok(())
    }

    async fn update_rule(
        &self,
        rule: &AutomationRule,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut rules = self.rules.lock().unwrap();
        let stored = rules
            .get(&rule.id)
            .filter(|stored| stored.team_id == rule.team_id);
        if stored.is_some_and(|stored| stored.updated_at != expected_updated_at) {
            return Err(DomainError::ConcurrentModification);
        }
        let exists = stored.is_some();
        if exists {
            rules.insert(rule.id, rule.clone());
        }
        Ok(exists)
    }

    async fn find_rule_for_team(
        &self,
        team_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Option<AutomationRule>, DomainError> {
        Ok(self
            .rules
            .lock()
            .unwrap()
            .get(&rule_id)
            .filter(|rule| rule.team_id == team_id)
            .cloned())
    }

    async fn list_rules_for_team(&self, team_id: Uuid) -> Result<Vec<AutomationRule>, DomainError> {
        let mut rules: Vec<_> = self
            .rules
            .lock()
            .unwrap()
            .values()
            .filter(|rule| rule.team_id == team_id)
            .cloned()
            .collect();
        rules.sort_by_key(|rule| (rule.created_at, rule.id));
        Ok(rules)
    }

    async fn list_enabled_rules_for_trigger(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
        trigger_kind: &str,
    ) -> Result<Vec<AutomationRule>, DomainError> {
        Ok(self
            .list_rules_for_team(team_id)
            .await?
            .into_iter()
            .filter(|rule| {
                rule.enabled
                    && rule.trigger_connection_id == connection_id
                    && rule.trigger_kind == trigger_kind
            })
            .collect())
    }

    async fn delete_rule(&self, team_id: Uuid, rule_id: Uuid) -> Result<bool, DomainError> {
        let mut rules = self.rules.lock().unwrap();
        let belongs_to_team = rules
            .get(&rule_id)
            .is_some_and(|rule| rule.team_id == team_id);
        if belongs_to_team {
            rules.remove(&rule_id);
        }
        Ok(belongs_to_team)
    }
}

#[derive(Default)]
pub struct DummyWebhookDeliveryRepo {
    deliveries: Mutex<HashMap<(Uuid, String), WebhookDelivery>>,
    claims: Mutex<HashMap<Uuid, Uuid>>,
}

#[allow(dead_code)]
impl DummyWebhookDeliveryRepo {
    pub fn all(&self) -> Vec<WebhookDelivery> {
        self.deliveries.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl WebhookDeliveryRepo for DummyWebhookDeliveryRepo {
    async fn claim_delivery(
        &self,
        delivery: &WebhookDelivery,
    ) -> Result<Option<opswarden_server::ports::WebhookDeliveryClaim>, DomainError> {
        let key = (
            delivery.connection_id,
            delivery.provider_delivery_id.clone(),
        );
        let mut deliveries = self.deliveries.lock().unwrap();
        if deliveries.contains_key(&key) {
            return Ok(None);
        }
        deliveries.insert(key, delivery.clone());
        let token = Uuid::new_v4();
        self.claims.lock().unwrap().insert(delivery.id, token);
        Ok(Some(opswarden_server::ports::WebhookDeliveryClaim {
            delivery_id: delivery.id,
            token,
        }))
    }

    async fn complete_claimed_delivery(
        &self,
        delivery: &WebhookDelivery,
        claim: opswarden_server::ports::WebhookDeliveryClaim,
    ) -> Result<bool, DomainError> {
        if self.claims.lock().unwrap().get(&delivery.id) != Some(&claim.token) {
            return Ok(false);
        }
        let updated = self.update_delivery(delivery).await?;
        if updated {
            self.claims.lock().unwrap().remove(&delivery.id);
        }
        Ok(updated)
    }

    async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<bool, DomainError> {
        let key = (
            delivery.connection_id,
            delivery.provider_delivery_id.clone(),
        );
        let mut deliveries = self.deliveries.lock().unwrap();
        let can_update = deliveries
            .get(&key)
            .is_some_and(|stored| stored.status.to_string() == "received");
        if can_update {
            deliveries.insert(key, delivery.clone());
        }
        Ok(can_update)
    }

    async fn list_deliveries_for_team(
        &self,
        _team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError> {
        let mut deliveries: Vec<_> = self.deliveries.lock().unwrap().values().cloned().collect();
        deliveries.sort_by_key(|delivery| std::cmp::Reverse(delivery.received_at));
        deliveries.truncate(limit.clamp(1, 200) as usize);
        Ok(deliveries)
    }
}

#[derive(Default)]
pub struct DummyAutomationRunRepo {
    runs: Mutex<HashMap<Uuid, AutomationRun>>,
}

#[allow(dead_code)]
impl DummyAutomationRunRepo {
    pub fn all(&self) -> Vec<AutomationRun> {
        self.runs.lock().unwrap().values().cloned().collect()
    }
}

#[async_trait]
impl AutomationRunRepo for DummyAutomationRunRepo {
    async fn insert_run(&self, run: &AutomationRun) -> Result<(), DomainError> {
        self.runs.lock().unwrap().insert(run.id, run.clone());
        Ok(())
    }

    async fn update_run(&self, run: &AutomationRun) -> Result<bool, DomainError> {
        let mut runs = self.runs.lock().unwrap();
        let can_update = runs
            .get(&run.id)
            .is_some_and(|stored| stored.status.to_string() == "running");
        if can_update {
            runs.insert(run.id, run.clone());
        }
        Ok(can_update)
    }

    async fn list_runs_for_team(
        &self,
        _team_id: Uuid,
        limit: u32,
    ) -> Result<Vec<AutomationRun>, DomainError> {
        let mut runs: Vec<_> = self.runs.lock().unwrap().values().cloned().collect();
        runs.sort_by_key(|run| std::cmp::Reverse(run.started_at));
        runs.truncate(limit.clamp(1, 200) as usize);
        Ok(runs)
    }
}

#[derive(Default)]
pub struct DummyNotifier {
    calls: Mutex<Vec<(String, String)>>,
    should_fail: Mutex<bool>,
}

#[allow(dead_code)]
impl DummyNotifier {
    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn fail_requests(&self) {
        *self.should_fail.lock().unwrap() = true;
    }
}

#[async_trait]
impl Notifier for DummyNotifier {
    async fn validate_endpoint(&self, _url: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_string(), message.to_string()));
        if *self.should_fail.lock().unwrap() {
            Err(DomainError::ReactionHttp5xx)
        } else {
            Ok(())
        }
    }
}

/// `DomainError` is deliberately not `Clone`, so the injected failure is stored
/// as a constructor the double can call on every attempt.
#[derive(Default)]
pub struct DummyEmailSender {
    sent: Mutex<Vec<(SmtpConfig, EmailMessage)>>,
    validated: Mutex<Vec<SmtpConfig>>,
    failure: Mutex<Option<fn() -> DomainError>>,
}

#[allow(dead_code)]
impl DummyEmailSender {
    pub fn sent(&self) -> Vec<(SmtpConfig, EmailMessage)> {
        self.sent.lock().unwrap().clone()
    }

    pub fn validated(&self) -> Vec<SmtpConfig> {
        self.validated.lock().unwrap().clone()
    }

    pub fn fail_with(&self, error: fn() -> DomainError) {
        *self.failure.lock().unwrap() = Some(error);
    }

    fn outcome(&self) -> Result<(), DomainError> {
        match *self.failure.lock().unwrap() {
            Some(error) => Err(error()),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl EmailSender for DummyEmailSender {
    async fn validate_smtp(&self, config: &SmtpConfig) -> Result<(), DomainError> {
        self.validated.lock().unwrap().push(config.clone());
        self.outcome()
    }

    async fn send_email(
        &self,
        config: &SmtpConfig,
        message: &EmailMessage,
    ) -> Result<(), DomainError> {
        // Record before failing so a test can assert what the executor built even
        // on the error path.
        self.sent
            .lock()
            .unwrap()
            .push((config.clone(), message.clone()));
        self.outcome()
    }
}

pub struct DummyGifSearch;

#[async_trait]
impl GifSearch for DummyGifSearch {
    async fn search(
        &self,
        query: &str,
        _limit: u32,
        _rating: &str,
    ) -> Result<Vec<GifResult>, DomainError> {
        Ok(vec![GifResult {
            id: "demo".to_string(),
            title: format!("result for {query}"),
            url: "https://media.giphy.com/media/demo/giphy.gif".to_string(),
            preview_url: "https://media.giphy.com/media/demo/200w_s.gif".to_string(),
            width: 200,
            height: 150,
        }])
    }
}
