//! Safe interpolation for Automation REAction payloads.
//!
//! Templates only see normalized, non-secret `ExternalEvent` attributes.
//! Credentials and raw provider payloads are deliberately unavailable.

use crate::domain::automation::ExternalEvent;
use crate::domain::error::DomainError;

pub const MAX_TEMPLATE_BYTES: usize = 1024;
pub const MAX_INTERPOLATED_TITLE_BYTES: usize = 200;
pub const MAX_INTERPOLATED_PAYLOAD_BYTES: usize = 1024;

const ALLOWED_VARIABLES: &[&str] = &[
    "repository",
    "workflow",
    "branch",
    "conclusion",
    "run_url",
    "tag",
    "commit_sha",
    "pull_request_number",
    "pull_request_title",
    "source_branch",
    "actor",
    "event_url",
    "release_id",
    "release_title",
    "release_state",
    "incident_id",
    "event_type",
    "source",
    "title",
    "message",
    "severity",
    "external_id",
    "alertname",
    "summary",
    "description",
    "receiver",
    "group_key",
    "instance",
    "namespace",
    "pod",
    "service",
    "job",
    "fingerprint",
    "starts_at",
    "ends_at",
    "generator_url",
    "status",
];

pub fn validate_template(template: &str) -> Result<(), DomainError> {
    if template.len() > MAX_TEMPLATE_BYTES {
        return Err(DomainError::InvalidAutomationRule);
    }
    template_parts(template).map(|_| ())
}

pub fn interpolate(
    template: &str,
    event: &ExternalEvent,
    max_output_bytes: usize,
) -> Result<String, DomainError> {
    let parts = template_parts(template)?;
    let mut output = String::with_capacity(template.len().min(max_output_bytes));
    for part in parts {
        match part {
            TemplatePart::Literal(value) => push_bounded(&mut output, value, max_output_bytes)?,
            TemplatePart::Variable(name) => {
                let value = event
                    .attributes
                    .get(name)
                    .and_then(serde_json::Value::as_str)
                    .ok_or(DomainError::InvalidAutomationRule)?;
                push_bounded(&mut output, value, max_output_bytes)?;
            }
        }
    }
    Ok(output)
}

enum TemplatePart<'a> {
    Literal(&'a str),
    Variable(&'a str),
}

fn template_parts(template: &str) -> Result<Vec<TemplatePart<'_>>, DomainError> {
    if template.len() > MAX_TEMPLATE_BYTES {
        return Err(DomainError::InvalidAutomationRule);
    }
    let mut parts = Vec::new();
    let mut cursor = 0;
    while let Some(relative_open) = template[cursor..].find("{{") {
        let open = cursor + relative_open;
        if template[cursor..open].contains("}}") {
            return Err(DomainError::InvalidAutomationRule);
        }
        if open > cursor {
            parts.push(TemplatePart::Literal(&template[cursor..open]));
        }
        let variable_start = open + 2;
        let relative_close = template[variable_start..]
            .find("}}")
            .ok_or(DomainError::InvalidAutomationRule)?;
        let close = variable_start + relative_close;
        let variable = template[variable_start..close].trim();
        if variable.contains("{{") || !ALLOWED_VARIABLES.contains(&variable) {
            return Err(DomainError::InvalidAutomationRule);
        }
        parts.push(TemplatePart::Variable(variable));
        cursor = close + 2;
    }
    if template[cursor..].contains("}}") {
        return Err(DomainError::InvalidAutomationRule);
    }
    if cursor < template.len() {
        parts.push(TemplatePart::Literal(&template[cursor..]));
    }
    Ok(parts)
}

fn push_bounded(
    output: &mut String,
    value: &str,
    max_output_bytes: usize,
) -> Result<(), DomainError> {
    if output.len().saturating_add(value.len()) > max_output_bytes {
        return Err(DomainError::ReactionPayloadTooLarge);
    }
    output.push_str(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};

    fn event() -> ExternalEvent {
        let attributes: Map<String, Value> = serde_json::from_value(json!({
            "repository": "opswarden/app",
            "workflow": "CI",
            "branch": "main",
            "oauth_access_token": "must-never-be-visible"
        }))
        .unwrap();
        ExternalEvent::new("github", "ci_failed").with_attributes(attributes)
    }

    #[test]
    fn interpolates_allowlisted_normalized_values_once() {
        assert_eq!(
            interpolate(
                "{{ workflow }} failed on {{repository}} ({{branch}})",
                &event(),
                MAX_INTERPOLATED_PAYLOAD_BYTES
            )
            .unwrap(),
            "CI failed on opswarden/app (main)"
        );

        let mut nested = event();
        nested.attributes.insert(
            "workflow".into(),
            Value::String("{{oauth_access_token}}".into()),
        );
        assert_eq!(
            interpolate("Run {{workflow}}", &nested, MAX_INTERPOLATED_PAYLOAD_BYTES).unwrap(),
            "Run {{oauth_access_token}}"
        );
    }

    #[test]
    fn rejects_unknown_secret_and_malformed_variables() {
        for template in [
            "{{token}}",
            "{{oauth_access_token}}",
            "{{unknown}}",
            "{{repository",
            "repository}}",
            "{{{{repository}}",
            "{{ }}",
        ] {
            assert_eq!(
                validate_template(template),
                Err(DomainError::InvalidAutomationRule),
                "{template}"
            );
        }
    }

    #[test]
    fn rejects_missing_values_and_bounded_output_overflow() {
        assert_eq!(
            interpolate("{{run_url}}", &event(), MAX_INTERPOLATED_PAYLOAD_BYTES),
            Err(DomainError::InvalidAutomationRule)
        );
        assert_eq!(
            interpolate("{{repository}}", &event(), 4),
            Err(DomainError::ReactionPayloadTooLarge)
        );
        assert_eq!(
            validate_template(&"x".repeat(MAX_TEMPLATE_BYTES + 1)),
            Err(DomainError::InvalidAutomationRule)
        );
    }

    #[test]
    fn accepts_extended_github_event_variables() {
        for template in [
            "Tag {{tag}} at {{commit_sha}} by {{actor}}: {{event_url}}",
            "PR #{{pull_request_number}} {{pull_request_title}} from {{source_branch}}",
        ] {
            assert_eq!(validate_template(template), Ok(()));
        }
    }

    #[test]
    fn accepts_native_opswarden_event_variables() {
        for template in [
            "Release {{release_title}} ({{release_id}}) is {{release_state}}",
            "Incident {{incident_id}}",
        ] {
            assert_eq!(validate_template(template), Ok(()));
        }
    }

    #[test]
    fn accepts_normalized_alertmanager_string_variables() {
        assert_eq!(
            validate_template(
                "{{alertname}} {{status}} on {{instance}}: {{summary}} ({{fingerprint}})"
            ),
            Ok(())
        );
    }
}
