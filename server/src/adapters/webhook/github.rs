// server/src/adapters/webhook/github.rs
//
// Maps GitHub webhook payloads onto domain `ExternalEvent`s. Provider-specific
// JSON shapes are an adapter concern and stay here, never in the app or domain.

use serde_json::Value;

use crate::domain::automation::ExternalEvent;
use crate::ports::WebhookParser;

/// Parser for the GitHub webhook events exposed by the automation catalog.
pub struct GithubParser;

impl WebhookParser for GithubParser {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "github" {
            return None;
        }

        let json: Value = serde_json::from_slice(body).ok()?;
        match provider_event {
            "workflow_run" => workflow_run_event(&json),
            "push" => tag_push_event(&json),
            "pull_request" => merged_pull_request_event(&json),
            _ => None,
        }
    }
}

fn workflow_run_event(payload: &Value) -> Option<ExternalEvent> {
    let conclusion = payload.pointer("/workflow_run/conclusion")?.as_str()?;
    let kind = match conclusion {
        "failure" | "timed_out" | "startup_failure" => "ci_failed",
        "success" => "ci_succeeded",
        _ => return None,
    };
    Some(ExternalEvent::new("github", kind).with_attributes(workflow_attributes(payload)))
}

fn tag_push_event(payload: &Value) -> Option<ExternalEvent> {
    let reference = payload.get("ref")?.as_str()?;
    let tag = reference.strip_prefix("refs/tags/")?;
    if tag.is_empty()
        || payload.get("created").and_then(Value::as_bool) != Some(true)
        || payload.get("deleted").and_then(Value::as_bool) == Some(true)
    {
        return None;
    }
    let mut attributes = serde_json::Map::new();
    insert_string(
        &mut attributes,
        "repository",
        payload.pointer("/repository/full_name"),
    );
    attributes.insert("tag".into(), Value::String(tag.to_string()));
    insert_string(&mut attributes, "commit_sha", payload.get("after"));
    insert_string(&mut attributes, "actor", payload.pointer("/sender/login"));
    insert_string(&mut attributes, "event_url", payload.get("compare"));
    Some(ExternalEvent::new("github", "tag_pushed").with_attributes(attributes))
}

fn merged_pull_request_event(payload: &Value) -> Option<ExternalEvent> {
    if payload.get("action").and_then(Value::as_str) != Some("closed")
        || payload
            .pointer("/pull_request/merged")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return None;
    }
    let mut attributes = serde_json::Map::new();
    insert_string(
        &mut attributes,
        "repository",
        payload.pointer("/repository/full_name"),
    );
    insert_number_as_string(
        &mut attributes,
        "pull_request_number",
        payload.get("number"),
    );
    insert_string(
        &mut attributes,
        "pull_request_title",
        payload.pointer("/pull_request/title"),
    );
    insert_string(
        &mut attributes,
        "branch",
        payload.pointer("/pull_request/base/ref"),
    );
    insert_string(
        &mut attributes,
        "source_branch",
        payload.pointer("/pull_request/head/ref"),
    );
    insert_string(
        &mut attributes,
        "actor",
        payload.pointer("/pull_request/merged_by/login"),
    );
    insert_string(
        &mut attributes,
        "event_url",
        payload.pointer("/pull_request/html_url"),
    );
    Some(ExternalEvent::new("github", "pr_merged").with_attributes(attributes))
}

fn workflow_attributes(payload: &Value) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    let fields = [
        ("repository", payload.pointer("/repository/full_name")),
        ("workflow", payload.pointer("/workflow_run/name")),
        ("branch", payload.pointer("/workflow_run/head_branch")),
        ("conclusion", payload.pointer("/workflow_run/conclusion")),
        ("run_url", payload.pointer("/workflow_run/html_url")),
    ];
    for (name, value) in fields {
        insert_string(&mut attributes, name, value);
    }
    attributes
}

fn insert_string(
    attributes: &mut serde_json::Map<String, Value>,
    name: &str,
    value: Option<&Value>,
) {
    if let Some(Value::String(value)) = value {
        attributes.insert(name.to_string(), Value::String(value.clone()));
    }
}

fn insert_number_as_string(
    attributes: &mut serde_json::Map<String, Value>,
    name: &str,
    value: Option<&Value>,
) {
    if let Some(value) = value.and_then(Value::as_u64) {
        attributes.insert(name.to_string(), Value::String(value.to_string()));
    }
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
    fn successful_workflow_run_becomes_ci_succeeded() {
        let body = br#"{"workflow_run":{"conclusion":"success"}}"#;
        let event = GithubParser.parse("github", "workflow_run", body).unwrap();
        assert_eq!(event.kind, "ci_succeeded");
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

    #[test]
    fn newly_created_tag_becomes_tag_pushed() {
        let body = br#"{
            "ref":"refs/tags/v1.2.3",
            "created":true,
            "deleted":false,
            "after":"abc123",
            "compare":"https://github.com/opswarden/app/compare/v1.2.3",
            "repository":{"full_name":"opswarden/app"},
            "sender":{"login":"octocat"}
        }"#;
        let event = GithubParser.parse("github", "push", body).unwrap();
        assert_eq!(event.kind, "tag_pushed");
        assert_eq!(event.attributes["tag"], "v1.2.3");
        assert_eq!(event.attributes["actor"], "octocat");
    }

    #[test]
    fn branch_or_existing_tag_push_is_ignored() {
        for body in [
            br#"{"ref":"refs/heads/main","created":true}"#.as_slice(),
            br#"{"ref":"refs/tags/v1.2.3","created":false}"#.as_slice(),
            br#"{"ref":"refs/tags/v1.2.3","created":true,"deleted":true}"#.as_slice(),
        ] {
            assert!(GithubParser.parse("github", "push", body).is_none());
        }
    }

    #[test]
    fn merged_pull_request_becomes_pr_merged() {
        let body = br#"{
            "action":"closed",
            "number":42,
            "repository":{"full_name":"opswarden/app"},
            "pull_request":{
                "merged":true,
                "title":"Ship VIGIL",
                "html_url":"https://github.com/opswarden/app/pull/42",
                "base":{"ref":"main"},
                "head":{"ref":"feature/vigil"},
                "merged_by":{"login":"octocat"}
            }
        }"#;
        let event = GithubParser.parse("github", "pull_request", body).unwrap();
        assert_eq!(event.kind, "pr_merged");
        assert_eq!(event.attributes["pull_request_number"], "42");
        assert_eq!(event.attributes["branch"], "main");
        assert_eq!(event.attributes["source_branch"], "feature/vigil");
    }

    #[test]
    fn closed_unmerged_or_open_pull_request_is_ignored() {
        for body in [
            br#"{"action":"closed","pull_request":{"merged":false}}"#.as_slice(),
            br#"{"action":"opened","pull_request":{"merged":false}}"#.as_slice(),
        ] {
            assert!(GithubParser.parse("github", "pull_request", body).is_none());
        }
    }
}
