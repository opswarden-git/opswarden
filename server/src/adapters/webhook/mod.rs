// --- server/src/adapters/webhook/mod.rs ---

pub mod alertmanager;
pub mod generic;
pub mod github;
pub mod gitlab;

pub use alertmanager::AlertmanagerParser;
pub use generic::GenericParser;
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
                Box::new(GenericParser),
                Box::new(AlertmanagerParser),
            ],
        }
    }
}

impl Default for CompositeWebhookParser {
    fn default() -> Self {
        Self::new()
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
