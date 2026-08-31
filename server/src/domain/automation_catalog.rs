//! Server-owned automation catalog shared by validation and `/about.json`.
mod alertmanager;

pub const GITHUB_SERVICE: &str = "github";
pub const GITLAB_SERVICE: &str = "gitlab";
pub const GENERIC_SERVICE: &str = "generic";
pub const ALERTMANAGER_SERVICE: &str = "alertmanager";
pub const OPSWARDEN_SERVICE: &str = "opswarden";
pub const TIMER_SERVICE: &str = "timer";
pub const HTTP_SERVICE: &str = "http";
pub const EMAIL_SERVICE: &str = "email";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionConfigurator {
    Github,
    TokenWebhook,
    Http,
    Email,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookAuthentication {
    Signature,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionProbe {
    Http,
    Email,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionExecutor {
    CreateIncident,
    ValidateReleaseStep,
    BlockRelease,
    EscalateIncident,
    HttpNotify,
    EmailNotify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogCapability {
    pub kind: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub connection_service: Option<&'static str>,
    pub fields: &'static [CatalogField],
    pub executor: Option<ReactionExecutor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogField {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub input_type: &'static str,
    pub required: bool,
    pub default_value: Option<&'static str>,
    pub options: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogOAuth {
    pub label: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogConnection {
    pub description: &'static str,
    pub fields: &'static [CatalogField],
    pub oauth: Option<CatalogOAuth>,
    pub configurator: ConnectionConfigurator,
    pub webhook_authentication: Option<WebhookAuthentication>,
    pub probe: Option<ConnectionProbe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomationServiceDefinition {
    pub service: &'static str,
    pub label: &'static str,
    pub actions: &'static [CatalogCapability],
    pub reactions: &'static [CatalogCapability],
    pub connection: Option<CatalogConnection>,
    pub internal: bool,
}

mod capabilities;
mod services;

pub use services::AUTOMATION_CATALOG;

pub fn service(name: &str) -> Option<&'static AutomationServiceDefinition> {
    AUTOMATION_CATALOG
        .iter()
        .find(|definition| definition.service == name)
}

pub fn action(service_name: &str, kind: &str) -> Option<&'static CatalogCapability> {
    service(service_name)?
        .actions
        .iter()
        .find(|action| action.kind == kind)
}

pub fn supports_action(service: &str, kind: &str) -> bool {
    action(service, kind).is_some()
}

pub fn reaction(kind: &str) -> Option<&'static CatalogCapability> {
    AUTOMATION_CATALOG
        .iter()
        .flat_map(|definition| definition.reactions)
        .find(|reaction| reaction.kind == kind)
}

pub fn reaction_executor(kind: &str) -> Option<ReactionExecutor> {
    reaction(kind)?.executor
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_resolves_action_and_reaction_connection_requirements() {
        assert!(supports_action("github", "ci_failed"));
        assert!(supports_action("github", "ci_succeeded"));
        assert!(supports_action("github", "tag_pushed"));
        assert!(supports_action("github", "pr_merged"));
        assert!(supports_action("gitlab", "ci_failed"));
        assert!(supports_action("gitlab", "ci_succeeded"));
        assert!(supports_action("gitlab", "tag_pushed"));
        assert!(supports_action("generic", "generic_event"));
        assert!(supports_action("alertmanager", "alert_firing"));
        assert!(supports_action("alertmanager", "alert_resolved"));
        assert!(supports_action("opswarden", "release_created"));
        assert!(supports_action("timer", "daily_at"));
        assert!(supports_action("timer", "every_minutes"));
        assert!(!supports_action("http", "ci_failed"));
        assert_eq!(
            reaction("create_incident").unwrap().connection_service,
            None
        );
        assert_eq!(
            reaction("http_notify").unwrap().connection_service,
            Some("http")
        );
        assert_eq!(
            AUTOMATION_CATALOG[0].connection.unwrap().fields[0].name,
            "webhook_signing_secret"
        );
        assert_eq!(reaction("create_incident").unwrap().fields.len(), 2);
        assert_eq!(reaction("validate_release_step").unwrap().fields.len(), 2);
        assert_eq!(reaction("block_release").unwrap().fields.len(), 3);
        assert_eq!(reaction("escalate_incident").unwrap().fields.len(), 1);
    }

    #[test]
    fn catalog_classifies_every_runtime_behavior() {
        for definition in AUTOMATION_CATALOG {
            assert!(definition
                .actions
                .iter()
                .all(|capability| capability.executor.is_none()));
            assert!(definition
                .reactions
                .iter()
                .all(|capability| capability.executor.is_some()));
            if definition.internal {
                assert!(definition.connection.is_none());
            }
            if let Some(connection) = definition.connection {
                assert_eq!(
                    connection.webhook_authentication.is_some(),
                    matches!(
                        connection.configurator,
                        ConnectionConfigurator::Github | ConnectionConfigurator::TokenWebhook
                    )
                );
            }
        }
        assert_eq!(
            service(GITHUB_SERVICE)
                .unwrap()
                .connection
                .unwrap()
                .webhook_authentication,
            Some(WebhookAuthentication::Signature)
        );
        assert_eq!(
            service(HTTP_SERVICE).unwrap().connection.unwrap().probe,
            Some(ConnectionProbe::Http)
        );
        assert!(service(OPSWARDEN_SERVICE).unwrap().internal);
        assert_eq!(
            reaction_executor("email_notify"),
            Some(ReactionExecutor::EmailNotify)
        );
    }
}
