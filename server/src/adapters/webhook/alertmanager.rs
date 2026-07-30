use std::collections::HashSet;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::domain::automation::ExternalEvent;
use crate::domain::error::DomainError;
use crate::ports::WebhookParser;

pub const MAX_ALERTMANAGER_BODY_BYTES: usize = 1024 * 1024;
const MAX_NORMALIZED_VALUE_BYTES: usize = 1024;
const TRANSITION_ID_VERSION: &str = "alertmanager-transition-v1";

pub struct AlertmanagerTransition {
    pub delivery_id: String,
    pub body: Vec<u8>,
}

pub struct AlertmanagerParser;

impl WebhookParser for AlertmanagerParser {
    fn parse(&self, service: &str, _provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "alertmanager" {
            return None;
        }
        parse_single_transition(body).ok()
    }
}

/// Validate one Alertmanager notification group and split it into independently
/// idempotent alert lifecycle transitions.
pub fn transitions(body: &[u8]) -> Result<Vec<AlertmanagerTransition>, DomainError> {
    if body.is_empty() || body.len() > MAX_ALERTMANAGER_BODY_BYTES {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| DomainError::InvalidWebhookDelivery)?;
    let object = payload
        .as_object()
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    required_status(object.get("status"))?;
    let group_key = required_string(object.get("groupKey"))?;
    let receiver = required_string(object.get("receiver"))?;
    let alerts = object
        .get("alerts")
        .and_then(Value::as_array)
        .filter(|alerts| !alerts.is_empty())
        .ok_or(DomainError::InvalidWebhookDelivery)?;

    let mut ids = HashSet::with_capacity(alerts.len());
    let mut result = Vec::with_capacity(alerts.len());
    for alert in alerts {
        let alert = validate_alert(alert)?;
        let status = required_status(alert.get("status"))?;
        let fingerprint = required_string(alert.get("fingerprint"))?;
        let starts_at = required_string(alert.get("startsAt"))?;
        let ends_at = if status == "resolved" {
            required_string(alert.get("endsAt"))?
        } else {
            ""
        };
        let delivery_id =
            semantic_delivery_id(group_key, receiver, status, fingerprint, starts_at, ends_at)?;
        if !ids.insert(delivery_id.clone()) {
            return Err(DomainError::InvalidWebhookDelivery);
        }
        result.push(AlertmanagerTransition {
            delivery_id,
            body: single_transition_body(object, alert, status)?,
        });
    }
    Ok(result)
}

fn validate_alert(alert: &Value) -> Result<&Map<String, Value>, DomainError> {
    let alert = alert
        .as_object()
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    for field in ["labels", "annotations"] {
        if alert.get(field).is_some_and(|value| !value.is_object()) {
            return Err(DomainError::InvalidWebhookDelivery);
        }
    }
    Ok(alert)
}

fn required_status(value: Option<&Value>) -> Result<&str, DomainError> {
    match required_string(value)? {
        status @ ("firing" | "resolved") => Ok(status),
        _ => Err(DomainError::InvalidWebhookDelivery),
    }
}

fn required_string(value: Option<&Value>) -> Result<&str, DomainError> {
    normalized_string(value).ok_or(DomainError::InvalidWebhookDelivery)
}

fn semantic_delivery_id(
    group_key: &str,
    receiver: &str,
    status: &str,
    fingerprint: &str,
    starts_at: &str,
    ends_at: &str,
) -> Result<String, DomainError> {
    let identity = serde_json::to_vec(&json!([
        TRANSITION_ID_VERSION,
        group_key,
        receiver,
        fingerprint,
        status,
        starts_at,
        ends_at
    ]))
    .map_err(|_| DomainError::InvalidWebhookDelivery)?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(identity))))
}

fn single_transition_body(
    group: &Map<String, Value>,
    alert: &Map<String, Value>,
    status: &str,
) -> Result<Vec<u8>, DomainError> {
    let mut payload = group.clone();
    payload.insert("status".into(), Value::String(status.to_string()));
    payload.insert(
        "alerts".into(),
        Value::Array(vec![Value::Object(alert.clone())]),
    );
    serde_json::to_vec(&Value::Object(payload)).map_err(|_| DomainError::InvalidWebhookDelivery)
}

fn parse_single_transition(body: &[u8]) -> Result<ExternalEvent, DomainError> {
    let payload: Value =
        serde_json::from_slice(body).map_err(|_| DomainError::InvalidWebhookDelivery)?;
    let object = payload
        .as_object()
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    let alerts = object
        .get("alerts")
        .and_then(Value::as_array)
        .filter(|alerts| alerts.len() == 1)
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    let alert = validate_alert(&alerts[0])?;
    let status = required_status(alert.get("status"))?;
    let kind = match status {
        "firing" => "alert_firing",
        "resolved" => "alert_resolved",
        _ => return Err(DomainError::InvalidWebhookDelivery),
    };
    Ok(ExternalEvent::new("alertmanager", kind)
        .with_attributes(normalized_attributes(object, alert, status)))
}

fn normalized_attributes(
    group: &Map<String, Value>,
    alert: &Map<String, Value>,
    status: &str,
) -> Map<String, Value> {
    let mut attributes = Map::new();
    attributes.insert("status".into(), Value::String(status.to_string()));
    copy_string(group.get("groupKey"), "group_key", &mut attributes);
    copy_string(group.get("receiver"), "receiver", &mut attributes);
    for field in ["fingerprint", "startsAt", "endsAt", "generatorURL"] {
        copy_string(
            alert.get(field),
            snake_case_alert_field(field),
            &mut attributes,
        );
    }
    for field in [
        "alertname",
        "severity",
        "instance",
        "namespace",
        "pod",
        "service",
        "job",
    ] {
        copy_nested_string(alert, "labels", field, &mut attributes);
    }
    for field in ["summary", "description"] {
        copy_nested_string(alert, "annotations", field, &mut attributes);
    }
    attributes
}

fn snake_case_alert_field(field: &str) -> &str {
    match field {
        "startsAt" => "starts_at",
        "endsAt" => "ends_at",
        "generatorURL" => "generator_url",
        _ => field,
    }
}

fn copy_nested_string(
    source: &Map<String, Value>,
    section: &str,
    field: &str,
    target: &mut Map<String, Value>,
) {
    let value = source
        .get(section)
        .and_then(Value::as_object)
        .and_then(|values| values.get(field));
    copy_string(value, field, target);
}

fn copy_string(value: Option<&Value>, target: &str, attributes: &mut Map<String, Value>) {
    if let Some(value) = normalized_string(value) {
        attributes.insert(target.to_string(), Value::String(value.to_string()));
    }
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
    use crate::ports::WebhookParser;

    const GROUP: &[u8] = br#"{
      "version":"4","status":"firing","receiver":"opswarden","groupKey":"api",
      "alerts":[
        {
          "status":"firing","fingerprint":"api-1",
          "startsAt":"2026-07-30T12:00:00Z","endsAt":"2026-07-30T12:05:00Z",
          "generatorURL":"https://prometheus.example/graph",
          "labels":{"severity":"critical","alertname":"ApiDown","instance":"api-1","token":"no"},
          "annotations":{"summary":"API unavailable","description":"Health probe failed","secret":"no"}
        },
        {
          "status":"resolved","fingerprint":"worker-1",
          "startsAt":"2026-07-30T11:00:00Z","endsAt":"2026-07-30T12:01:00Z",
          "labels":{"severity":"warning","alertname":"WorkerDown"},
          "annotations":{"summary":"Worker recovered"}
        }
      ]
    }"#;

    #[test]
    fn mixed_group_becomes_one_transition_per_alert() {
        let transitions = transitions(GROUP).unwrap();
        assert_eq!(transitions.len(), 2);

        let firing = AlertmanagerParser
            .parse("alertmanager", "webhook", &transitions[0].body)
            .unwrap();
        assert_eq!(firing.kind, "alert_firing");
        assert_eq!(firing.attributes["fingerprint"], "api-1");
        assert_eq!(firing.attributes["alertname"], "ApiDown");
        assert_eq!(firing.attributes["description"], "Health probe failed");
        assert_eq!(firing.attributes["starts_at"], "2026-07-30T12:00:00Z");
        assert!(!firing.attributes.contains_key("token"));
        assert!(!firing.attributes.contains_key("secret"));

        let resolved = AlertmanagerParser
            .parse("alertmanager", "webhook", &transitions[1].body)
            .unwrap();
        assert_eq!(resolved.kind, "alert_resolved");
        assert_eq!(resolved.attributes["fingerprint"], "worker-1");
        assert_eq!(resolved.attributes["status"], "resolved");
    }

    #[test]
    fn semantic_id_ignores_json_formatting_and_firing_ends_at_changes() {
        let compact = br#"{"status":"firing","receiver":"opswarden","groupKey":"api","alerts":[{"status":"firing","fingerprint":"api-1","startsAt":"2026-07-30T12:00:00Z","endsAt":"2026-07-30T12:05:00Z"}]}"#;
        let changed_ends_at = br#"{
          "groupKey":"api","receiver":"opswarden","status":"firing",
          "alerts":[{"endsAt":"2026-07-30T12:10:00Z","startsAt":"2026-07-30T12:00:00Z","fingerprint":"api-1","status":"firing"}]
        }"#;
        assert_eq!(
            transitions(compact).unwrap()[0].delivery_id,
            transitions(changed_ends_at).unwrap()[0].delivery_id
        );
    }

    #[test]
    fn lifecycle_transitions_have_distinct_ids() {
        let transitions = transitions(GROUP).unwrap();
        assert_ne!(transitions[0].delivery_id, transitions[1].delivery_id);

        let later_firing = br#"{"status":"firing","receiver":"opswarden","groupKey":"api","alerts":[{"status":"firing","fingerprint":"api-1","startsAt":"2026-07-31T12:00:00Z"}]}"#;
        assert_ne!(
            transitions[0].delivery_id,
            super::transitions(later_firing).unwrap()[0].delivery_id
        );
    }

    #[test]
    fn rejects_empty_unknown_or_ambiguous_transitions() {
        for body in [
            br#"{"status":"firing","receiver":"opswarden","groupKey":"api","alerts":[]}"#.as_slice(),
            br#"{"status":"unknown","receiver":"opswarden","groupKey":"api","alerts":[]}"#.as_slice(),
            br#"{"status":"firing","receiver":"opswarden","groupKey":"api","alerts":[{"status":"firing","startsAt":"2026-07-30T12:00:00Z"}]}"#.as_slice(),
            br#"{"status":"resolved","receiver":"opswarden","groupKey":"api","alerts":[{"status":"resolved","fingerprint":"api-1","startsAt":"2026-07-30T12:00:00Z"}]}"#.as_slice(),
        ] {
            assert_eq!(
                transitions(body).map(|_| ()),
                Err(DomainError::InvalidWebhookDelivery)
            );
        }
    }
}
