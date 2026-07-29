use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::domain::automation::ExternalEvent;
use crate::domain::automation_config::{AutomationRule, CredentialKind};
use crate::domain::automation_template::{
    interpolate, MAX_INTERPOLATED_PAYLOAD_BYTES, MAX_INTERPOLATED_TITLE_BYTES,
};
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::{Incident, Severity};
use crate::domain::incident_event::IncidentEvent;
use crate::domain::release::ReleaseState;
use crate::ports::{
    ConnectionCredentialVault, EmailMessage, EmailSender, EventPublisher, IncidentRepo, Notifier,
    ReleaseRepo, ServiceConnectionRepo, SmtpConfig,
};

const HTTP_SERVICE: &str = "http";
const EMAIL_SERVICE: &str = "email";
const MAX_NOTIFICATION_TEXT_BYTES: usize = 1024;

pub struct AutomationReactionExecutor {
    connections: Arc<dyn ServiceConnectionRepo>,
    credentials: Arc<dyn ConnectionCredentialVault>,
    incidents: Arc<dyn IncidentRepo>,
    releases: Arc<dyn ReleaseRepo>,
    notifier: Arc<dyn Notifier>,
    events: Arc<dyn EventPublisher>,
    email_sender: Arc<dyn EmailSender>,
}

impl AutomationReactionExecutor {
    pub fn new(
        connections: Arc<dyn ServiceConnectionRepo>,
        credentials: Arc<dyn ConnectionCredentialVault>,
        incidents: Arc<dyn IncidentRepo>,
        releases: Arc<dyn ReleaseRepo>,
        notifier: Arc<dyn Notifier>,
        events: Arc<dyn EventPublisher>,
        email_sender: Arc<dyn EmailSender>,
    ) -> Self {
        Self {
            connections,
            credentials,
            incidents,
            releases,
            notifier,
            events,
            email_sender,
        }
    }

    pub async fn execute(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        match rule.reaction_kind.as_str() {
            "create_incident" => self.create_incident(team_id, rule, event).await,
            "validate_release_step" => self.validate_release_step(team_id, rule, event).await,
            "block_release" => self.block_release(team_id, rule, event).await,
            "escalate_incident" => self.escalate_incident(team_id, rule, event).await,
            "http_notify" | "slack_notify" => self.notify_http(team_id, rule, event).await,
            "email_notify" => self.notify_email(team_id, rule, event).await,
            _ => Err(DomainError::InvalidAutomationRule),
        }
    }

    async fn validate_release_step(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        let release_id = configured_uuid(&rule.reaction_config, "release_id", event)?;
        let step = configured_required_text(&rule.reaction_config, "step", event)?;
        let actor = rule.created_by.ok_or(DomainError::InvalidAutomationRule)?;
        let mut release = self
            .releases
            .find_release_by_id(release_id)
            .await?
            .filter(|release| release.team_id == team_id)
            .ok_or(DomainError::ReleaseNotFound)?;
        let has_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let old_state = release.effective_state(has_active);
        release.validate_step(&step, actor, has_active)?;
        self.releases.update_release(&release).await?;
        self.events
            .publish(DomainEvent::ReleaseStepValidated {
                team_id,
                release_id,
                step,
                by: actor,
            })
            .await;
        let new_state = release.effective_state(has_active);
        if new_state != old_state {
            self.events
                .publish(DomainEvent::ReleaseStateChanged {
                    team_id,
                    release_id,
                    new_state,
                })
                .await;
        }
        Ok(None)
    }

    async fn block_release(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        let release_id = configured_uuid(&rule.reaction_config, "release_id", event)?;
        let release = self
            .releases
            .find_release_by_id(release_id)
            .await?
            .filter(|release| release.team_id == team_id)
            .ok_or(DomainError::ReleaseNotFound)?;
        let has_active = self
            .releases
            .count_active_linked_incidents(release.id)
            .await?
            > 0;
        let old_state = release.effective_state(has_active);
        if release.base_state != ReleaseState::InProgress {
            return Err(DomainError::InvalidReleaseTransition);
        }
        if old_state == ReleaseState::Blocked {
            return Err(DomainError::ReleaseBlocked);
        }

        let severity = configured_severity(&rule.reaction_config)?;
        let title = configured_title(&rule.reaction_config, event)?
            .unwrap_or_else(|| format!("Automation blocked {}", release.title));
        let incident =
            Incident::new_with_description(team_id, title, incident_description(event), severity)?;
        let created = IncidentEvent::created(&incident, None);
        self.incidents
            .save_incident_with_event(&incident, &created)
            .await?;
        self.releases.link_incident(release.id, incident.id).await?;
        let new_state = release.effective_state(true);
        if new_state != old_state {
            self.events
                .publish(DomainEvent::ReleaseStateChanged {
                    team_id,
                    release_id,
                    new_state,
                })
                .await;
        }
        Ok(Some((incident.id, incident.severity)))
    }

    async fn escalate_incident(
        &self,
        team_id: Uuid,
        rule: &AutomationRule,
        event: &ExternalEvent,
    ) -> Result<Option<(Uuid, Severity)>, DomainError> {
        let incident_id = configured_uuid(&rule.reaction_config, "incident_id", event)?;
        let actor = rule.created_by.ok_or(DomainError::InvalidAutomationRule)?;
        let mut incident = self
            .incidents
            .find_incident_by_id(incident_id)
            .await?
            .filter(|incident| incident.team_id == team_id)
            .ok_or(DomainError::IncidentNotFound)?;
        let previous = incident.status;
        if incident.escalate()? {
            let audit =
                IncidentEvent::status_changed(incident.id, actor, previous, incident.status);
            self.incidents
                .update_incident_with_event(&incident, &audit)
                .await?;
            self.events
                .publish(DomainEvent::IncidentStateChanged {
                    team_id,
                    incident_id,
                    new_status: incident.status,
                    by: actor,
                })
                .await;
            self.events
                .publish(DomainEvent::IncidentEscalated {
                    team_id,
                    incident_id,
                    new_severity: incident.severity,
                    by: actor,
                })
                .await;
        }
        Ok(None)
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
        let message = configured_message_by_name(&rule.reaction_config, "message", event)?
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

    async fn notify_email(
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
        if connection.service != EMAIL_SERVICE {
            return Err(DomainError::InvalidAutomationRule);
        }

        let config = smtp_config(self.credentials.as_ref(), connection.id).await?;
        let message = EmailMessage {
            to: configured_required_text(&rule.reaction_config, "to", event)?,
            subject: configured_message_by_name(&rule.reaction_config, "subject", event)?
                .unwrap_or_else(|| notification_text(event)),
            body: configured_message_by_name(&rule.reaction_config, "body", event)?
                .unwrap_or_else(|| notification_text(event)),
        };

        match self.email_sender.send_email(&config, &message).await {
            Ok(()) => {
                self.connections
                    .record_reaction_result(connection.id, None)
                    .await?;
                Ok(None)
            }
            // Record the actual failure code: a malformed recipient must not be
            // reported to the Manager as an SMTP outage.
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

/// Decrypt the five SMTP credentials of an email connection. Shared by the
/// REAction and the Manager-facing connection test so both read the same vault
/// contract and fail identically on a half-configured connection.
pub(crate) async fn smtp_config(
    credentials: &dyn ConnectionCredentialVault,
    connection_id: Uuid,
) -> Result<SmtpConfig, DomainError> {
    let reveal = |kind: CredentialKind| async move {
        credentials
            .reveal_credential(connection_id, kind)
            .await?
            .ok_or(DomainError::InvalidReactionEndpoint)
    };

    Ok(SmtpConfig {
        host: reveal(CredentialKind::SmtpHost).await?,
        port: reveal(CredentialKind::SmtpPort)
            .await?
            .parse::<u16>()
            .map_err(|_| DomainError::InvalidReactionEndpoint)?,
        username: reveal(CredentialKind::SmtpUsername).await?,
        password: reveal(CredentialKind::SmtpPassword).await?,
        from: reveal(CredentialKind::FromAddress).await?,
    })
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

fn configured_message_by_name(
    config: &Value,
    field: &str,
    event: &ExternalEvent,
) -> Result<Option<String>, DomainError> {
    interpolate_config_field(config, field, event, MAX_INTERPOLATED_PAYLOAD_BYTES)
}

fn configured_required_text(
    config: &Value,
    field: &str,
    event: &ExternalEvent,
) -> Result<String, DomainError> {
    interpolate_config_field(config, field, event, MAX_INTERPOLATED_PAYLOAD_BYTES)?
        .ok_or(DomainError::InvalidAutomationRule)
}

fn configured_uuid(
    config: &Value,
    field: &str,
    event: &ExternalEvent,
) -> Result<Uuid, DomainError> {
    configured_required_text(config, field, event)?
        .parse()
        .map_err(|_| DomainError::InvalidAutomationRule)
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
    let repository = attribute(event, "repository").unwrap_or("OpsWarden");
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
        "release_created" => {
            let title = attribute(event, "release_title").unwrap_or("Release");
            format!("Release {title} created")
        }
        "generic_event" => attribute(event, "title")
            .map(str::to_string)
            .unwrap_or_else(|| {
                let event_type = attribute(event, "event_type").unwrap_or("generic");
                format!("Generic event: {event_type}")
            }),
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
        ("Release", attribute(event, "release_id")),
        ("Release title", attribute(event, "release_title")),
        ("Release state", attribute(event, "release_state")),
        ("Incident", attribute(event, "incident_id")),
        ("Event type", attribute(event, "event_type")),
        ("Source", attribute(event, "source")),
        ("Title", attribute(event, "title")),
        ("Message", attribute(event, "message")),
        ("Severity", attribute(event, "severity")),
        ("External ID", attribute(event, "external_id")),
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
            configured_message_by_name(
                &json!({"message": "{{workflow}} failed on {{repository}}"}),
                "message",
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
