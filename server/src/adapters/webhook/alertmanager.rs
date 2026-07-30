use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::domain::automation::ExternalEvent;
use crate::domain::error::DomainError;
use crate::ports::WebhookParser;

pub const MAX_ALERTMANAGER_BODY_BYTES: usize = 1024 * 1024;
const FIRING_EVENT: &str = "alert_firing";
const MAX_NORMALIZED_VALUE_BYTES: usize = 1024;

pub struct AlertmanagerParser;

impl WebhookParser for AlertmanagerParser {
    fn parse(&self, service: &str, _provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "alertmanager" {
            return None;
        }
        let payload = validate_payload(body).ok()?;
        if payload.get("status").and_then(Value::as_str) != Some("firing") {
            return None;
        }
        Some(
            ExternalEvent::new("alertmanager", FIRING_EVENT)
                .with_attributes(normalized_attributes(&payload)),
        )
    }
}

/// Reject malformed notifications before authentication state or delivery rows
/// are touched. Alertmanager sends one notification group per request.
pub fn validate_payload(body: &[u8]) -> Result<Value, DomainError> {
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| DomainError::InvalidWebhookDelivery)?;
    let object = payload
        .as_object()
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    if !matches!(status, "firing" | "resolved") {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    let alerts = object
        .get("alerts")
        .and_then(Value::as_array)
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    if status == "firing" && alerts.is_empty() {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    if alerts.iter().any(|alert| !valid_alert(alert)) {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    Ok(payload)
}

/// Alertmanager has no delivery-id header. A digest makes exact retries
/// idempotent while still accepting an updated notification for the same group.
pub fn delivery_id(body: &[u8]) -> Result<String, DomainError> {
    validate_payload(body)?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(body))))
}

fn valid_alert(alert: &Value) -> bool {
    let Some(alert) = alert.as_object() else {
        return false;
    };
    alert.get("status").is_none_or(Value::is_string)
        && alert.get("labels").is_none_or(Value::is_object)
        && alert.get("annotations").is_none_or(Value::is_object)
}

fn normalized_attributes(payload: &Value) -> Map<String, Value> {
    let mut attributes = Map::new();
    let alerts = payload["alerts"].as_array().expect("validated alerts");
    attributes.insert("status".to_string(), Value::String("firing".to_string()));
    attributes.insert("alert_count".to_string(), Value::from(alerts.len()));
    attributes.insert("alerts".to_string(), normalized_alerts(alerts));

    copy_string(payload, "groupKey", "group_key", &mut attributes);
    copy_string(payload, "receiver", "receiver", &mut attributes);
    copy_common_value(payload, "commonLabels", "severity", &mut attributes);
    copy_common_value(payload, "commonLabels", "alertname", &mut attributes);
    copy_common_value(payload, "commonAnnotations", "summary", &mut attributes);
    copy_shared_alert_value(alerts, "labels", "severity", &mut attributes);
    copy_shared_alert_value(alerts, "labels", "alertname", &mut attributes);
    copy_shared_alert_value(alerts, "annotations", "summary", &mut attributes);
    attributes
}

fn copy_string(payload: &Value, source: &str, target: &str, attributes: &mut Map<String, Value>) {
    if let Some(value) = normalized_string(payload.get(source)) {
        attributes.insert(target.to_string(), Value::String(value.to_string()));
    }
}

fn copy_common_value(payload: &Value, field: &str, key: &str, attributes: &mut Map<String, Value>) {
    let value = payload
        .get(field)
        .and_then(Value::as_object)
        .and_then(|values| values.get(key));
    if let Some(value) = normalized_string(value) {
        attributes
            .entry(key.to_string())
            .or_insert_with(|| Value::String(value.to_string()));
    }
}

fn copy_shared_alert_value(
    alerts: &[Value],
    field: &str,
    key: &str,
    attributes: &mut Map<String, Value>,
) {
    let first = alerts
        .first()
        .and_then(|alert| alert.get(field))
        .and_then(Value::as_object)
        .and_then(|values| values.get(key));
    let Some(first) = normalized_string(first) else {
        return;
    };
    let shared = alerts.iter().all(|alert| {
        alert
            .get(field)
            .and_then(Value::as_object)
            .and_then(|values| values.get(key))
            .and_then(|value| normalized_string(Some(value)))
            == Some(first)
    });
    if shared {
        attributes
            .entry(key.to_string())
            .or_insert_with(|| Value::String(first.to_string()));
    }
}

fn normalized_alerts(alerts: &[Value]) -> Value {
    Value::Array(
        alerts
            .iter()
            .map(|alert| {
                let mut normalized = Map::new();
                for field in ["status", "fingerprint"] {
                    if let Some(value) = normalized_string(alert.get(field)) {
                        normalized.insert(field.to_string(), Value::String(value.to_string()));
                    }
                }
                for (section, field) in [
                    ("labels", "alertname"),
                    ("labels", "severity"),
                    ("annotations", "summary"),
                ] {
                    let value = alert
                        .get(section)
                        .and_then(Value::as_object)
                        .and_then(|values| values.get(field));
                    if let Some(value) = normalized_string(value) {
                        normalized.insert(field.to_string(), Value::String(value.to_string()));
                    }
                }
                Value::Object(normalized)
            })
            .collect(),
    )
}

fn normalized_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= MAX_NORMALIZED_VALUE_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUP: &[u8] = br#"{
        "version":"4","status":"firing","receiver":"opswarden","groupKey":"{}:{severity=\"critical\"}",
        "commonLabels":{"severity":"critical"},
        "alerts":[
            {"status":"firing","fingerprint":"abc123","labels":{"severity":"critical","alertname":"ApiDown","token":"must-not-leak"},"annotations":{"summary":"API unavailable","secret":"must-not-leak"}},
            {"status":"firing","labels":{"severity":"critical","alertname":"WorkerDown"},"annotations":{"summary":"Worker unavailable"}}
        ]
    }"#;

    #[test]
    fn firing_group_keeps_every_alert_and_only_shared_flat_attributes() {
        let event = AlertmanagerParser
            .parse("alertmanager", "webhook", GROUP)
            .expect("firing event");
        assert_eq!(event.kind, FIRING_EVENT);
        assert_eq!(event.attributes["alert_count"], 2);
        assert_eq!(event.attributes["severity"], "critical");
        assert_eq!(event.attributes["receiver"], "opswarden");
        let alerts = event.attributes["alerts"].as_array().unwrap();
        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0]["alertname"], "ApiDown");
        assert_eq!(alerts[0]["summary"], "API unavailable");
        assert_eq!(alerts[0]["fingerprint"], "abc123");
        assert!(alerts[0].get("token").is_none());
        assert!(alerts[0].get("secret").is_none());
        assert!(!event.attributes.contains_key("alertname"));
        assert!(!event.attributes.contains_key("summary"));
    }

    #[test]
    fn resolved_group_is_valid_but_does_not_trigger_a_rule() {
        let body = br#"{"status":"resolved","alerts":[]}"#;
        assert!(validate_payload(body).is_ok());
        assert!(AlertmanagerParser
            .parse("alertmanager", "webhook", body)
            .is_none());
    }

    #[test]
    fn malformed_or_empty_firing_payload_is_rejected() {
        assert_eq!(
            validate_payload(br#"{"status":"firing","alerts":[]}"#),
            Err(DomainError::InvalidWebhookDelivery)
        );
        assert_eq!(
            validate_payload(b"not-json"),
            Err(DomainError::InvalidWebhookDelivery)
        );
    }

    #[test]
    fn exact_retries_share_a_stable_delivery_id() {
        assert_eq!(delivery_id(GROUP).unwrap(), delivery_id(GROUP).unwrap());
        assert_ne!(
            delivery_id(GROUP).unwrap(),
            delivery_id(br#"{"status":"resolved","alerts":[]}"#).unwrap()
        );
    }
}
