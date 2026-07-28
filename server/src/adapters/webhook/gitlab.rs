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
                if json.get("object_kind").and_then(Value::as_str) != Some("pipeline") {
                    return None;
                }
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
            "Tag Push Hook" => tag_push_event(&json),
            _ => None,
        }
    }
}

fn gitlab_attributes(payload: &Value) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    let fields = [
        (
            "repository",
            payload.pointer("/project/path_with_namespace"),
        ),
        // In GitLab, the "workflow" equivalent is often the pipeline name or stages. We'll fallback to a generic name.
        ("workflow", payload.pointer("/object_attributes/name")),
        ("branch", payload.pointer("/object_attributes/ref")),
        ("conclusion", payload.pointer("/object_attributes/status")),
        ("run_url", payload.pointer("/object_attributes/url")),
    ];
    for (name, value) in fields {
        if let Some(Value::String(value)) = value {
            attributes.insert(name.to_string(), Value::String(value.clone()));
        }
    }
    attributes
}

fn tag_push_event(payload: &Value) -> Option<ExternalEvent> {
    if payload.get("object_kind").and_then(Value::as_str) != Some("tag_push") {
        return None;
    }
    let reference = payload.get("ref")?.as_str()?;
    let tag = reference.strip_prefix("refs/tags/")?;
    let before = payload.get("before")?.as_str()?;
    let after = payload.get("after")?.as_str()?;
    if tag.is_empty() || !is_git_oid(before, true) || !is_git_oid(after, false) {
        return None;
    }
    let mut attributes = serde_json::Map::new();
    insert_string(
        &mut attributes,
        "repository",
        payload.pointer("/project/path_with_namespace"),
    );
    attributes.insert("tag".into(), Value::String(tag.to_string()));
    attributes.insert("commit_sha".into(), Value::String(after.to_string()));
    insert_string(&mut attributes, "actor", payload.get("user_username"));
    insert_string(
        &mut attributes,
        "event_url",
        payload.pointer("/project/web_url"),
    );
    Some(ExternalEvent::new("gitlab", "tag_pushed").with_attributes(attributes))
}

fn is_git_oid(value: &str, zero: bool) -> bool {
    matches!(value.len(), 40 | 64)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && if zero {
            value.bytes().all(|byte| byte == b'0')
        } else {
            value.bytes().any(|byte| byte != b'0')
        }
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
        let body = br#"{"object_kind":"pipeline","object_attributes":{"status":"success"}}"#;
        let event = GitlabParser.parse("gitlab", "Pipeline Hook", body).unwrap();
        assert_eq!(event.kind, "ci_succeeded");
    }

    #[test]
    fn tag_push_becomes_tag_pushed() {
        let body = br#"{
            "object_kind":"tag_push",
            "ref":"refs/tags/v1.2.3",
            "before":"0000000000000000000000000000000000000000",
            "after":"abcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "user_username":"octocat",
            "project":{"path_with_namespace":"opswarden/app","web_url":"https://gitlab.com/opswarden/app"}
        }"#;
        let event = GitlabParser.parse("gitlab", "Tag Push Hook", body).unwrap();
        assert_eq!(event.kind, "tag_pushed");
        assert_eq!(event.attributes["tag"], "v1.2.3");
        assert_eq!(event.attributes["actor"], "octocat");
    }

    #[test]
    fn updated_or_deleted_tag_is_ignored() {
        for body in [
            br#"{"object_kind":"tag_push","ref":"refs/tags/v1","before":"abc","after":"def"}"#
                .as_slice(),
            br#"{"object_kind":"tag_push","ref":"refs/tags/v1","before":"0000","after":"0000"}"#
                .as_slice(),
        ] {
            assert!(GitlabParser
                .parse("gitlab", "Tag Push Hook", body)
                .is_none());
        }
    }
}
