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
    async fn reserve_run(
        &self,
        run: &AutomationRun,
        _claim: opswarden_server::ports::WebhookDeliveryClaim,
    ) -> Result<opswarden_server::ports::AutomationRunReservation, DomainError> {
        let mut runs = self.runs.lock().unwrap();
        if let Some(existing) = runs.values().find(|existing| {
            existing.delivery_id == run.delivery_id && existing.rule_id == run.rule_id
        }) {
            return Ok(opswarden_server::ports::AutomationRunReservation::Existing(
                existing.clone(),
            ));
        }
        runs.insert(run.id, run.clone());
        Ok(opswarden_server::ports::AutomationRunReservation::New(
            run.clone(),
        ))
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

    async fn interrupt_running_for_delivery(
        &self,
        claim: opswarden_server::ports::WebhookDeliveryClaim,
    ) -> Result<u64, DomainError> {
        let mut runs = self.runs.lock().unwrap();
        let mut interrupted = 0;
        for run in runs.values_mut().filter(|run| {
            run.delivery_id == claim.delivery_id
                && run.status
                    == opswarden_server::domain::automation_config::AutomationRunStatus::Running
        }) {
            run.mark_failed("interrupted")?;
            interrupted += 1;
        }
        Ok(interrupted)
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
