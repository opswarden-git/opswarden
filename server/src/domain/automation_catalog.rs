//! Server-owned catalog for Team automation.
//!
//! Stored rule kinds, API validation and `/about.json` all consume this same
//! registry. That keeps the future UI descriptive instead of hard-coded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogCapability {
    pub kind: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub connection_service: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomationServiceDefinition {
    pub service: &'static str,
    pub label: &'static str,
    pub actions: &'static [CatalogCapability],
    pub reactions: &'static [CatalogCapability],
}

const GITHUB_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "ci_failed",
    label: "CI run failed",
    description: "A GitHub Actions workflow run completed with a failing conclusion",
    connection_service: Some("github"),
}];

const VIGIL_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "vigil_create_incident",
    label: "Create incident",
    description: "Open an incident in the Team that owns the automation rule",
    connection_service: None,
}];

const HTTP_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "http_notify",
    label: "Send HTTP notification",
    description: "Send a notification through a configured HTTP connection",
    connection_service: Some("http"),
}];

pub const AUTOMATION_CATALOG: &[AutomationServiceDefinition] = &[
    AutomationServiceDefinition {
        service: "github",
        label: "GitHub",
        actions: GITHUB_ACTIONS,
        reactions: &[],
    },
    AutomationServiceDefinition {
        service: "vigil",
        label: "VIGIL",
        actions: &[],
        reactions: VIGIL_REACTIONS,
    },
    AutomationServiceDefinition {
        service: "http",
        label: "HTTP",
        actions: &[],
        reactions: HTTP_REACTIONS,
    },
];

pub fn supports_action(service: &str, kind: &str) -> bool {
    AUTOMATION_CATALOG.iter().any(|definition| {
        definition.service == service && definition.actions.iter().any(|action| action.kind == kind)
    })
}

pub fn reaction(kind: &str) -> Option<&'static CatalogCapability> {
    AUTOMATION_CATALOG
        .iter()
        .flat_map(|definition| definition.reactions)
        .find(|reaction| reaction.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_resolves_action_and_reaction_connection_requirements() {
        assert!(supports_action("github", "ci_failed"));
        assert!(!supports_action("http", "ci_failed"));
        assert_eq!(
            reaction("vigil_create_incident")
                .unwrap()
                .connection_service,
            None
        );
        assert_eq!(
            reaction("http_notify").unwrap().connection_service,
            Some("http")
        );
    }
}
