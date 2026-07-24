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
    pub fields: &'static [CatalogField],
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
    pub testable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomationServiceDefinition {
    pub service: &'static str,
    pub label: &'static str,
    pub actions: &'static [CatalogCapability],
    pub reactions: &'static [CatalogCapability],
    pub connection: Option<CatalogConnection>,
}

const NO_OPTIONS: &[&str] = &[];
const SEVERITY_OPTIONS: &[&str] = &["low", "medium", "high", "critical"];

const CI_FILTERS: &[CatalogField] = &[
    CatalogField {
        name: "repository",
        label: "Repository",
        description: "Only match this repository",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "workflow",
        label: "Workflow",
        description: "Only match this workflow",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "branch",
        label: "Branch",
        description: "Only match this branch",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "conclusion",
        label: "Conclusion",
        description: "Only match this workflow conclusion",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const GITHUB_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "ci_failed",
    label: "CI run failed",
    description: "A GitHub Actions workflow run completed with a failing conclusion",
    connection_service: Some("github"),
    fields: CI_FILTERS,
}];

const INCIDENT_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "severity",
        label: "Severity",
        description: "Severity assigned to the created incident",
        input_type: "select",
        required: true,
        default_value: Some("high"),
        options: SEVERITY_OPTIONS,
    },
    CatalogField {
        name: "title",
        label: "Incident title",
        description: "Optional template using {{repository}}, {{workflow}}, {{branch}}, {{conclusion}} or {{run_url}}",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const VIGIL_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "vigil_create_incident",
    label: "Create incident",
    description: "Open an incident in the Team that owns the automation rule",
    connection_service: None,
    fields: INCIDENT_FIELDS,
}];

const HTTP_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "http_notify",
    label: "Send HTTP notification",
    description: "Send a notification through a configured HTTP connection",
    connection_service: Some("http"),
    fields: &[CatalogField {
        name: "message",
        label: "Message",
        description:
            "Template using {{repository}}, {{workflow}}, {{branch}}, {{conclusion}} or {{run_url}}",
        input_type: "text",
        required: false,
        default_value: Some("{{workflow}} failed on {{repository}}"),
        options: NO_OPTIONS,
    }],
}];

const GITHUB_CONNECTION_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "webhook_signing_secret",
        label: "Webhook signing secret",
        description: "Required on first connection; leave blank later to preserve it",
        input_type: "password",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "personal_token",
        label: "Personal access token",
        description: "Optional encrypted alternative to OAuth",
        input_type: "password",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const HTTP_CONNECTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "endpoint_url",
    label: "Endpoint URL",
    description: "Public HTTPS destination; local networks and credentialed URLs are rejected",
    input_type: "url",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

pub const AUTOMATION_CATALOG: &[AutomationServiceDefinition] = &[
    AutomationServiceDefinition {
        service: "github",
        label: "GitHub",
        actions: GITHUB_ACTIONS,
        reactions: &[],
        connection: Some(CatalogConnection {
            description: "Verify incoming webhooks and optionally authorize GitHub API access",
            fields: GITHUB_CONNECTION_FIELDS,
            oauth: Some(CatalogOAuth {
                label: "Authorize with GitHub",
                description: "Access and refresh tokens remain encrypted on the server",
            }),
            testable: false,
        }),
    },
    AutomationServiceDefinition {
        service: "vigil",
        label: "VIGIL",
        actions: &[],
        reactions: VIGIL_REACTIONS,
        connection: None,
    },
    AutomationServiceDefinition {
        service: "http",
        label: "HTTP",
        actions: &[],
        reactions: HTTP_REACTIONS,
        connection: Some(CatalogConnection {
            description: "Send bounded notifications to a public HTTPS endpoint",
            fields: HTTP_CONNECTION_FIELDS,
            oauth: None,
            testable: true,
        }),
    },
];

pub fn action(service: &str, kind: &str) -> Option<&'static CatalogCapability> {
    AUTOMATION_CATALOG
        .iter()
        .find(|definition| definition.service == service)?
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
        assert_eq!(
            AUTOMATION_CATALOG[0].connection.unwrap().fields[0].name,
            "webhook_signing_secret"
        );
        assert_eq!(reaction("vigil_create_incident").unwrap().fields.len(), 2);
    }
}
