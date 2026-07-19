//! Normalized provider input consumed by durable Team automation rules.

use serde_json::{Map, Value};

/// A provider event stripped down to non-secret facts understood by the rule
/// engine. Raw webhook bodies never enter the domain or persistence layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEvent {
    pub service: String,
    pub kind: String,
    pub attributes: Map<String, Value>,
}

impl ExternalEvent {
    pub fn new(service: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            kind: kind.into(),
            attributes: Map::new(),
        }
    }

    pub fn with_attributes(mut self, attributes: Map<String, Value>) -> Self {
        self.attributes = attributes;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_normalized_event_with_provider_attributes() {
        let mut attributes = Map::new();
        attributes.insert("repository".into(), Value::String("opswarden/app".into()));

        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);

        assert_eq!(event.service, "github");
        assert_eq!(event.kind, "ci_failed");
        assert_eq!(event.attributes["repository"], "opswarden/app");
    }
}
