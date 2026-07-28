use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{AutomationRule, CredentialKind};
use crate::domain::automation_template::{
    interpolate, MAX_INTERPOLATED_PAYLOAD_BYTES, MAX_INTERPOLATED_TITLE_BYTES,
};
use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, Severity};
use crate::domain::incident_event::IncidentEvent;
use crate::ports::{ConnectionCredentialVault, IncidentRepo, Notifier, ServiceConnectionRepo};

const HTTP_SERVICE: &str = "http";
const MAX_NOTIFICATION_TEXT_BYTES: usize = 1024;

pub struct AutomationReactionExecutor {
    connections: Arc<dyn ServiceConnectionRepo>,
    credentials: Arc<dyn ConnectionCredentialVault>,
    incidents: Arc<dyn IncidentRepo>,
    notifier: Arc<dyn Notifier>,
}

impl AutomationReactionExecutor {
    pub fn new(
        connections: Arc<dyn ServiceConnectionRepo>,
        credentials: Arc<dyn ConnectionCredentialVault>,
        incidents: Arc<dyn IncidentRepo>,
        notifier: Arc<dyn Notifier>,
    ) -> Self {
        Self {
            connections,
            credentials,
            incidents,
            notifier,
        }
    }

    pub async fn execute(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        match rule.reaction_kind.as_str() {
            "vigil_create_incident" => self.create_incident(team_id, rule, event).await,
            "http_notify" | "slack_notify" => self.notify_http(team_id, rule, event).await,
            _ => Err(DomainError::InvalidAutomationRule),
        }
    }

    async fn create_incident(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        let severity = configured_severity(&rule.reaction_config)?;
        let title = configured_title(&rule.reaction_config, event)?
            .unwrap_or_else(|| default_incident_title(event));
        let incident =
            Incident::new_with_description(team_id, title, incident_description(event), severity)?;
        let created = IncidentEvent::created(&incident, None);
        self.incidents
            .save_incident_with_event(&incident, &created)
            .await?;
        Ok(Some((incident.id, incident.severity)))
    }

    async fn notify_http(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        let connection_id = rule
            .reaction_connection_id
            .ok_or(DomainError::InvalidAutomationRule)?;
        let connection = self
            .connections
            .find_connection_for_team(team_id, connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if connection.service != HTTP_SERVICE && connection.service != "slack" {
            return Err(DomainError::InvalidAutomationRule);
        }
        let endpoint = self
            .credentials
            .reveal_credential(connection.id, CredentialKind::EndpointUrl)
            .await?
            .ok_or(DomainError::InvalidReactionEndpoint)?;
        let message = configured_message(&rule.reaction_config, event)?
            .unwrap_or_else(|| notification_text(event));

        match self.notifier.notify(&endpoint, &message).await {
            Ok(()) => {
                self.connections
                    .record_reaction_result(connection.id, None)
                    .await?;
                Ok(None)
            }
            Err(error) => {
                let _ = self
                    .connections
                    .record_reaction_result(connection.id, Some(error.code()))
                    .await;
                Err(error)
            }
        }
    }
}

fn configured_severity(config: &Value) -> Result<Severity, DomainError> {
    match config.get("severity").and_then(Value::as_str) {
        None | Some("high") => Ok(Severity::High),
        Some("low") => Ok(Severity::Low),
        Some("medium") => Ok(Severity::Medium),
        Some("critical") => Ok(Severity::Critical),
        Some(_) => Err(DomainError::InvalidSeverity),
    }
}

fn configured_title(config: &Value, event: &ExternalEvent) -> Result<Option<String>, DomainError> {
    interpolate_config_field(config, "title", event, MAX_INTERPOLATED_TITLE_BYTES)
}

fn configured_message(
    config: &Value,
    event: &ExternalEvent,
) -> Result<Option<String>, DomainError> {
    interpolate_config_field(config, "message", event, MAX_INTERPOLATED_PAYLOAD_BYTES)
}

fn interpolate_config_field(
    config: &Value,
    field: &str,
    event: &ExternalEvent,
    max_output_bytes: usize,
) -> Result<Option<String>, DomainError> {
    let Some(template) = config.get(field).and_then(Value::as_str) else {
        return Ok(None);
    };
    if template.trim().is_empty() {
        return Ok(None);
    }
    let value = interpolate(template, event, max_output_bytes)?;
    if value.trim().is_empty() {
        return Err(DomainError::InvalidAutomationRule);
    }
    Ok(Some(value))
}

fn attribute<'a>(event: &'a ExternalEvent, name: &str) -> Option<&'a str> {
    event.attributes.get(name).and_then(Value::as_str)
}

fn default_incident_title(event: &ExternalEvent) -> String {
    let repository = attribute(event, "repository").unwrap_or("GitHub");
    match event.kind.as_str() {
        "ci_failed" => {
            let workflow = attribute(event, "workflow").unwrap_or("CI");
            format!("{workflow} failed on {repository}")
        }
        "ci_succeeded" => {
            let workflow = attribute(event, "workflow").unwrap_or("CI");
            format!("{workflow} succeeded on {repository}")
        }
        "tag_pushed" => {
            let tag = attribute(event, "tag").unwrap_or("unknown");
            format!("Tag {tag} pushed on {repository}")
        }
        "pr_merged" => {
            let number = attribute(event, "pull_request_number")
                .map(|number| format!(" #{number}"))
                .unwrap_or_default();
            format!("Pull request{number} merged on {repository}")
        }
        _ => format!("Automation event on {repository}"),
    }
}

fn incident_description(event: &ExternalEvent) -> String {
    event_lines(event).join("\n")
}

fn notification_text(event: &ExternalEvent) -> String {
    let mut text = default_incident_title(event);
    let details = event_lines(event);
    if !details.is_empty() {
        text.push('\n');
        text.push_str(&details.join("\n"));
    }
    truncate_utf8(text, MAX_NOTIFICATION_TEXT_BYTES)
}

fn event_lines(event: &ExternalEvent) -> Vec<String> {
    [
        ("Repository", attribute(event, "repository")),
        ("Workflow", attribute(event, "workflow")),
        ("Branch", attribute(event, "branch")),
        ("Conclusion", attribute(event, "conclusion")),
        ("Run", attribute(event, "run_url")),
        ("Tag", attribute(event, "tag")),
        ("Commit", attribute(event, "commit_sha")),
        ("Pull request", attribute(event, "pull_request_number")),
        ("Title", attribute(event, "pull_request_title")),
        ("Source branch", attribute(event, "source_branch")),
        ("Actor", attribute(event, "actor")),
        ("Event", attribute(event, "event_url")),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
    .collect()
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes.saturating_sub('…'.len_utf8());
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value.push('…');
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    #[test]
    fn incident_and_notification_use_normalized_provider_facts() {
        let attributes: Map<String, Value> = serde_json::from_value(json!({
            "repository": "opswarden/app",
            "workflow": "CI",
            "branch": "main",
            "conclusion": "failure",
            "run_url": "https://github.test/run/42"
        }))
        .unwrap();
        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
        assert_eq!(default_incident_title(&event), "CI failed on opswarden/app");
        assert!(incident_description(&event).contains("Branch: main"));
        assert!(notification_text(&event).contains("Run: https://github.test/run/42"));
    }

    #[test]
    fn notification_text_is_utf8_safe_and_bounded() {
        let mut attributes = Map::new();
        attributes.insert("repository".into(), Value::String("é".repeat(2000)));
        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
        let text = notification_text(&event);
        assert!(text.len() <= MAX_NOTIFICATION_TEXT_BYTES);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn reaction_payload_templates_use_repository_and_workflow() {
        let attributes: Map<String, Value> = serde_json::from_value(json!({
            "repository": "opswarden/app",
            "workflow": "CI"
        }))
        .unwrap();
        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);
        assert_eq!(
            configured_title(
                &json!({"title": "[{{repository}}] {{workflow}} failed"}),
                &event
            )
            .unwrap()
            .as_deref(),
            Some("[opswarden/app] CI failed")
        );
        assert_eq!(
            configured_message(
                &json!({"message": "{{workflow}} failed on {{repository}}"}),
                &event
            )
            .unwrap()
            .as_deref(),
            Some("CI failed on opswarden/app")
        );
    }

    #[test]
    fn extended_github_events_have_meaningful_default_titles() {
        let cases = [
            (
                "ci_succeeded",
                json!({"repository": "opswarden/app", "workflow": "CI"}),
                "CI succeeded on opswarden/app",
            ),
            (
                "tag_pushed",
                json!({"repository": "opswarden/app", "tag": "v1.2.3"}),
                "Tag v1.2.3 pushed on opswarden/app",
            ),
            (
                "pr_merged",
                json!({"repository": "opswarden/app", "pull_request_number": "42"}),
                "Pull request #42 merged on opswarden/app",
            ),
        ];
        for (kind, attributes, expected) in cases {
            let attributes: Map<String, Value> = serde_json::from_value(attributes).unwrap();
            let event = ExternalEvent::new("github", kind).with_attributes(attributes);
            assert_eq!(default_incident_title(&event), expected);
        }
    }
}
