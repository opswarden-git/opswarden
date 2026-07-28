// --- server/src/adapters/webhook/mod.rs ---

pub mod alertmanager;
pub mod github;
pub mod gitlab;

pub use alertmanager::AlertmanagerParser;
pub use github::GithubParser;
pub use gitlab::GitlabParser;

use crate::domain::automation::ExternalEvent;
use crate::ports::WebhookParser;

pub struct CompositeWebhookParser {
    parsers: Vec<Box<dyn WebhookParser>>,
}

impl CompositeWebhookParser {
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(GithubParser),
                Box::new(GitlabParser),
                Box::new(AlertmanagerParser),
            ],
        }
    }
}

impl WebhookParser for CompositeWebhookParser {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        for parser in &self.parsers {
            if let Some(event) = parser.parse(service, provider_event, body) {
                return Some(event);
            }
        }
        None
    }
}
