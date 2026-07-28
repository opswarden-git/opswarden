// server/src/adapters/webhook/alertmanager.rs

use serde_json::Value;

use crate::domain::automation::ExternalEvent;
use crate::ports::WebhookParser;

pub struct AlertmanagerParser;

impl WebhookParser for AlertmanagerParser {
    fn parse(&self, service: &str, _provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != "alertmanager" {
            return None;
        }

        let json: Value = serde_json::from_slice(body).ok()?;
        
        // Alertmanager payloads typically have a root "status" and an array of "alerts"
        let status = json.get("status").and_then(Value::as_str)?;

        if status == "firing" {
            Some(ExternalEvent::new("alertmanager", "alert_firing").with_attributes(alertmanager_attributes(&json)))
        } else {
            None
        }
    }
}

fn alertmanager_attributes(payload: &Value) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    
    // We try to extract from the first alert in the array
    if let Some(alerts) = payload.get("alerts").and_then(Value::as_array) {
        if let Some(first_alert) = alerts.first() {
            if let Some(Value::String(severity)) = first_alert.pointer("/labels/severity") {
                attributes.insert("severity".to_string(), Value::String(severity.clone()));
            }
            if let Some(Value::String(alertname)) = first_alert.pointer("/labels/alertname") {
                attributes.insert("alertname".to_string(), Value::String(alertname.clone()));
            }
            if let Some(Value::String(summary)) = first_alert.pointer("/annotations/summary") {
                attributes.insert("summary".to_string(), Value::String(summary.clone()));
            }
        }
    }
    
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firing_alert_becomes_alert_firing() {
        let body = br#"{"status":"firing","alerts":[{"labels":{"severity":"critical","alertname":"PodCrash"},"annotations":{"summary":"A pod crashed"}}]}"#;
        let event = AlertmanagerParser.parse("alertmanager", "webhook", body).unwrap();
        assert_eq!(event.service, "alertmanager");
        assert_eq!(event.kind, "alert_firing");
        assert_eq!(event.attributes["severity"], "critical");
        assert_eq!(event.attributes["alertname"], "PodCrash");
        assert_eq!(event.attributes["summary"], "A pod crashed");
    }

    #[test]
    fn resolved_alert_is_ignored() {
        let body = br#"{"status":"resolved","alerts":[]}"#;
        assert!(AlertmanagerParser.parse("alertmanager", "webhook", body).is_none());
    }
}
