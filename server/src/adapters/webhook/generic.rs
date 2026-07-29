//! Bounded mapping for provider-neutral JSON webhooks.
//!
//! The raw payload may contain arbitrary JSON, but automation only receives a
//! small allow-list of top-level string attributes. Unknown fields are ignored
//! and no JSONPath or executable expression is evaluated.

use serde_json::{Map, Value};

use crate::domain::automation::ExternalEvent;
use crate::domain::error::DomainError;
use crate::ports::WebhookParser;

pub const MAX_GENERIC_BODY_BYTES: usize = 64 * 1024;
const MAX_GENERIC_DEPTH: usize = 4;
const MAX_GENERIC_COLLECTION_ITEMS: usize = 32;
const MAX_GENERIC_KEY_BYTES: usize = 100;
const MAX_GENERIC_STRING_BYTES: usize = 1024;
const MAPPED_FIELDS: &[&str] = &[
    "source",
    "title",
    "message",
    "severity",
    "external_id",
    "event_url",
];

pub struct GenericParser;

impl WebhookParser for GenericParser {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "generic" {
            return None;
        }
        let payload = parse_and_validate(body).ok()?;
        let mut attributes = Map::new();
        attributes.insert(
            "event_type".into(),
            Value::String(provider_event.to_string()),
        );
        for field in MAPPED_FIELDS {
            if let Some(value) = payload.get(*field) {
                attributes.insert((*field).to_string(), value.clone());
            }
        }
        Some(ExternalEvent::new("generic", "generic_event").with_attributes(attributes))
    }
}

pub fn validate_payload(body: &[u8]) -> Result<(), DomainError> {
    parse_and_validate(body).map(|_| ())
}

fn parse_and_validate(body: &[u8]) -> Result<Map<String, Value>, DomainError> {
    if body.is_empty() || body.len() > MAX_GENERIC_BODY_BYTES {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    let value: Value =
        serde_json::from_slice(body).map_err(|_| DomainError::InvalidWebhookDelivery)?;
    validate_value(&value, 1)?;
    let object = value
        .as_object()
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    for field in MAPPED_FIELDS {
        if let Some(value) = object.get(*field) {
            let text = value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or(DomainError::InvalidWebhookDelivery)?;
            if *field == "severity" && !["low", "medium", "high", "critical"].contains(&text) {
                return Err(DomainError::InvalidWebhookDelivery);
            }
        }
    }
    Ok(object.clone())
}

fn validate_value(value: &Value, depth: usize) -> Result<(), DomainError> {
    if depth > MAX_GENERIC_DEPTH {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    match value {
        Value::String(value) if value.len() > MAX_GENERIC_STRING_BYTES => {
            Err(DomainError::InvalidWebhookDelivery)
        }
        Value::Array(values) => {
            if values.len() > MAX_GENERIC_COLLECTION_ITEMS {
                return Err(DomainError::InvalidWebhookDelivery);
            }
            for value in values {
                validate_value(value, depth + 1)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            if values.len() > MAX_GENERIC_COLLECTION_ITEMS
                || values.keys().any(|key| key.len() > MAX_GENERIC_KEY_BYTES)
            {
                return Err(DomainError::InvalidWebhookDelivery);
            }
            for value in values.values() {
                validate_value(value, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_only_bounded_top_level_strings() {
        let event = GenericParser
            .parse(
                "generic",
                "deployment_failed",
                br#"{"source":"jury","title":"Deploy failed","severity":"critical","ignored":{"token":"never mapped"}}"#,
            )
            .unwrap();
        assert_eq!(event.kind, "generic_event");
        assert_eq!(event.attributes["event_type"], "deployment_failed");
        assert_eq!(event.attributes["source"], "jury");
        assert_eq!(event.attributes["severity"], "critical");
        assert!(!event.attributes.contains_key("ignored"));
    }

    #[test]
    fn rejects_malformed_unbounded_or_ambiguous_payloads() {
        for body in [
            b"[]".as_slice(),
            b"not-json".as_slice(),
            br#"{"title":42}"#.as_slice(),
            br#"{"severity":"urgent"}"#.as_slice(),
            br#"{"a":{"b":{"c":{"d":true}}}}"#.as_slice(),
        ] {
            assert_eq!(
                validate_payload(body),
                Err(DomainError::InvalidWebhookDelivery)
            );
        }
        let oversized = format!(r#"{{"message":"{}"}}"#, "x".repeat(1025));
        assert_eq!(
            validate_payload(oversized.as_bytes()),
            Err(DomainError::InvalidWebhookDelivery)
        );
    }

    #[test]
    fn unrelated_service_is_ignored() {
        assert!(GenericParser.parse("github", "push", b"{}").is_none());
    }
}
