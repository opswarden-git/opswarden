//! Normalized provider input consumed by durable Team automation rules.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{automation_catalog::OPSWARDEN_SERVICE, release::Release};

/// A provider event stripped down to non-secret facts understood by the rule
/// engine. Raw webhook bodies never enter the domain or persistence layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn release_created_event(release: &Release) -> ExternalEvent {
    let mut attributes = Map::new();
    attributes.insert("release_id".into(), Value::String(release.id.to_string()));
    attributes.insert("release_title".into(), Value::String(release.title.clone()));
    attributes.insert(
        "release_state".into(),
        Value::String(release.base_state.to_string()),
    );
    ExternalEvent::new(OPSWARDEN_SERVICE, "release_created").with_attributes(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn builds_a_normalized_event_with_provider_attributes() {
        let mut attributes = Map::new();
        attributes.insert("repository".into(), Value::String("opswarden/app".into()));

        let event = ExternalEvent::new("github", "ci_failed").with_attributes(attributes);

        assert_eq!(event.service, "github");
        assert_eq!(event.kind, "ci_failed");
        assert_eq!(event.attributes["repository"], "opswarden/app");
    }

    #[test]
    fn release_created_contains_only_normalized_release_facts() {
        let release = Release::new(Uuid::new_v4(), "v2.0.0", vec!["build".to_string()]).unwrap();
        let event = release_created_event(&release);

        assert_eq!(event.service, OPSWARDEN_SERVICE);
        assert_eq!(event.kind, "release_created");
        assert_eq!(event.attributes["release_id"], release.id.to_string());
        assert_eq!(event.attributes["release_title"], "v2.0.0");
        assert_eq!(event.attributes["release_state"], "created");
    }
}
