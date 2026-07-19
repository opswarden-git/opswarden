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
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "github" || provider_event != "workflow_run" {
            return None;
        }

        let json: Value = serde_json::from_slice(body).ok()?;
        let conclusion = json
            .get("workflow_run")
            .and_then(|run| run.get("conclusion"))
            .and_then(Value::as_str)?;

        match conclusion {
            "failure" | "timed_out" | "startup_failure" => Some(
                ExternalEvent::new("github", "ci_failed").with_attributes(github_attributes(&json)),
            ),
            _ => None,
        }
    }
}

fn github_attributes(payload: &Value) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    let fields = [
        ("repository", payload.pointer("/repository/full_name")),
        ("workflow", payload.pointer("/workflow_run/name")),
        ("branch", payload.pointer("/workflow_run/head_branch")),
        ("conclusion", payload.pointer("/workflow_run/conclusion")),
        ("run_url", payload.pointer("/workflow_run/html_url")),
    ];
    for (name, value) in fields {
        if let Some(Value::String(value)) = value {
            attributes.insert(name.to_string(), Value::String(value.clone()));
        }
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_workflow_run_becomes_ci_failed() {
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        let event = GithubParser.parse("github", "workflow_run", body).unwrap();
        assert_eq!(event.service, "github");
        assert_eq!(event.kind, "ci_failed");
        assert_eq!(event.attributes["conclusion"], "failure");
    }

    #[test]
    fn successful_workflow_run_is_ignored() {
        let body = br#"{"workflow_run":{"conclusion":"success"}}"#;
        assert!(GithubParser.parse("github", "workflow_run", body).is_none());
    }

    #[test]
    fn unrelated_service_or_garbage_is_ignored() {
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        assert!(GithubParser.parse("gitlab", "workflow_run", body).is_none());
        assert!(GithubParser.parse("github", "push", body).is_none());
        assert!(GithubParser
            .parse("github", "workflow_run", b"not json")
            .is_none());
        assert!(GithubParser
            .parse("github", "workflow_run", b"{}")
            .is_none());
    }

    #[test]
    fn failed_run_exposes_normalized_filter_attributes() {
        let body = br#"{
            "repository":{"full_name":"opswarden/app"},
            "workflow_run":{
                "name":"CI",
                "head_branch":"main",
                "conclusion":"failure",
                "html_url":"https://github.com/opswarden/app/actions/runs/42"
            }
        }"#;
        let event = GithubParser.parse("github", "workflow_run", body).unwrap();
        assert_eq!(event.attributes["repository"], "opswarden/app");
        assert_eq!(event.attributes["workflow"], "CI");
        assert_eq!(event.attributes["branch"], "main");
        assert_eq!(event.attributes["conclusion"], "failure");
        assert_eq!(
            event.attributes["run_url"],
            "https://github.com/opswarden/app/actions/runs/42"
        );
    }
}
