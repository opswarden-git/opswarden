// server/src/adapters/webhook/gitlab.rs

use serde_json::Value;

use crate::domain::automation::ExternalEvent;
use crate::ports::WebhookParser;

pub struct GitlabParser;

impl WebhookParser for GitlabParser {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "gitlab" {
            return None;
        }

        let json: Value = serde_json::from_slice(body).ok()?;

        match provider_event {
            "Pipeline Hook" => {
                let status = json
                    .pointer("/object_attributes/status")
                    .and_then(Value::as_str)?;

                let kind = match status {
                    "failed" | "canceled" => "ci_failed",
                    "success" => "ci_succeeded",
                    _ => return None,
                };

                Some(ExternalEvent::new("gitlab", kind).with_attributes(gitlab_attributes(&json)))
            }
            "Tag Push Hook" => Some(
                ExternalEvent::new("gitlab", "tag_pushed").with_attributes(gitlab_attributes(&json)),
            ),
            _ => None,
        }
    }
}

fn gitlab_attributes(payload: &Value) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    let fields = [
        ("repository", payload.pointer("/project/path_with_namespace")),
        // In GitLab, the "workflow" equivalent is often the pipeline name or stages. We'll fallback to a generic name.
        ("workflow", payload.pointer("/object_attributes/name")),
        ("branch", payload.pointer("/object_attributes/ref")),
        ("conclusion", payload.pointer("/object_attributes/status")),
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
    fn failed_pipeline_becomes_ci_failed() {
        let body = br#"{"object_kind":"pipeline","object_attributes":{"status":"failed","ref":"main"},"project":{"path_with_namespace":"opswarden/app"}}"#;
        let event = GitlabParser.parse("gitlab", "Pipeline Hook", body).unwrap();
        assert_eq!(event.service, "gitlab");
        assert_eq!(event.kind, "ci_failed");
        assert_eq!(event.attributes["repository"], "opswarden/app");
        assert_eq!(event.attributes["conclusion"], "failed");
        assert_eq!(event.attributes["branch"], "main");
    }

    #[test]
    fn success_pipeline_becomes_ci_succeeded() {
        let body = br#"{"object_attributes":{"status":"success"}}"#;
        let event = GitlabParser.parse("gitlab", "Pipeline Hook", body).unwrap();
        assert_eq!(event.kind, "ci_succeeded");
    }

    #[test]
    fn tag_push_becomes_tag_pushed() {
        let body = br#"{"object_kind":"tag_push","project":{"path_with_namespace":"opswarden/app"}}"#;
        let event = GitlabParser.parse("gitlab", "Tag Push Hook", body).unwrap();
        assert_eq!(event.kind, "tag_pushed");
    }
}
