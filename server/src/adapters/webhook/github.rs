// server/src/adapters/webhook/github.rs
//
// Maps GitHub webhook payloads onto domain `ExternalEvent`s. Provider-specific
// JSON shapes are an adapter concern and stay here, never in the app or domain.

use serde_json::Value;

use crate::domain::automation::ExternalEvent;
use crate::ports::WebhookParser;

/// Parser for GitHub webhooks. Phase 2 surfaces one signal: a `workflow_run`
/// that completed with a failing conclusion becomes a `ci_failed` event.
pub struct GithubParser;

impl WebhookParser for GithubParser {
    fn parse(&self, service: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "github" {
            return None;
        }

        let json: Value = serde_json::from_slice(body).ok()?;
        let conclusion = json
            .get("workflow_run")
            .and_then(|run| run.get("conclusion"))
            .and_then(Value::as_str)?;

        match conclusion {
            "failure" | "timed_out" | "startup_failure" => {
                Some(ExternalEvent::new("github", "ci_failed"))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_workflow_run_becomes_ci_failed() {
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        let event = GithubParser.parse("github", body).unwrap();
        assert_eq!(event, ExternalEvent::new("github", "ci_failed"));
    }

    #[test]
    fn successful_workflow_run_is_ignored() {
        let body = br#"{"workflow_run":{"conclusion":"success"}}"#;
        assert!(GithubParser.parse("github", body).is_none());
    }

    #[test]
    fn unrelated_service_or_garbage_is_ignored() {
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        assert!(GithubParser.parse("gitlab", body).is_none());
        assert!(GithubParser.parse("github", b"not json").is_none());
        assert!(GithubParser.parse("github", b"{}").is_none());
    }
}
