use crate::domain::{automation::ExternalEvent, automation_catalog::OPSWARDEN_SERVICE};
use crate::ports::WebhookParser;

pub struct OpsWardenParser;

impl WebhookParser for OpsWardenParser {
    fn parse(&self, service: &str, provider_event: &str, body: &[u8]) -> Option<ExternalEvent> {
        if service != OPSWARDEN_SERVICE {
            return None;
        }
        let event: ExternalEvent = serde_json::from_slice(body).ok()?;
        (event.service == service && event.kind == provider_event).then_some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_matching_normalized_opswarden_events() {
        let event = ExternalEvent::new(OPSWARDEN_SERVICE, "release_created");
        let body = serde_json::to_vec(&event).unwrap();

        assert_eq!(
            OpsWardenParser.parse(OPSWARDEN_SERVICE, "release_created", &body),
            Some(event)
        );
        assert!(OpsWardenParser
            .parse(OPSWARDEN_SERVICE, "other", &body)
            .is_none());
        assert!(OpsWardenParser
            .parse("generic", "release_created", &body)
            .is_none());
    }
}
