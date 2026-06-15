// --- server/src/adapters/notify.rs ---
//
// Outbound HTTP notifier (the `Notify` REAction). POSTs `{"text": message}` to
// the target URL — that is the Slack incoming-webhook contract, which also works
// for Discord, Teams, and any endpoint accepting a JSON body. One connector,
// many targets: the rule just carries a `url`.

use async_trait::async_trait;
use serde_json::json;

use crate::domain::error::DomainError;
use crate::ports::Notifier;

pub struct HttpNotifier {
    client: reqwest::Client,
}

impl HttpNotifier {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError> {
        let response = self
            .client
            .post(url)
            .json(&json!({ "text": message }))
            .send()
            .await
            .map_err(|_| DomainError::ReactionFailed)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(DomainError::ReactionFailed)
        }
    }
}
