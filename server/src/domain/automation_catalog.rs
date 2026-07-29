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

const TAG_FILTERS: &[CatalogField] = &[
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
        name: "tag",
        label: "Tag",
        description: "Only match this exact tag",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const PULL_REQUEST_FILTERS: &[CatalogField] = &[
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
        name: "branch",
        label: "Target branch",
        description: "Only match pull requests merged into this branch",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "source_branch",
        label: "Source branch",
        description: "Only match pull requests from this branch",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const RELEASE_FILTERS: &[CatalogField] = &[
    CatalogField {
        name: "release_id",
        label: "Release ID",
        description: "Only match this Release",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "release_title",
        label: "Release title",
        description: "Only match this exact Release title",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const GENERIC_FILTERS: &[CatalogField] = &[
    CatalogField {
        name: "event_type",
        label: "Event type",
        description: "Only match the X-OpsWarden-Event value",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "source",
        label: "Source",
        description: "Only match this top-level payload source",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "severity",
        label: "Severity",
        description: "Only match this severity",
        input_type: "select",
        required: false,
        default_value: None,
        options: SEVERITY_OPTIONS,
    },
    CatalogField {
        name: "external_id",
        label: "External ID",
        description: "Only match this top-level external identifier",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const GITHUB_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "ci_failed",
        label: "CI run failed",
        description: "A GitHub Actions workflow run completed with a failing conclusion",
        connection_service: Some("github"),
        fields: CI_FILTERS,
    },
    CatalogCapability {
        kind: "ci_succeeded",
        label: "CI run succeeded",
        description: "A GitHub Actions workflow run completed successfully",
        connection_service: Some("github"),
        fields: CI_FILTERS,
    },
    CatalogCapability {
        kind: "tag_pushed",
        label: "New tag pushed",
        description: "A new Git tag was pushed to the repository",
        connection_service: Some("github"),
        fields: TAG_FILTERS,
    },
    CatalogCapability {
        kind: "pr_merged",
        label: "Pull request merged",
        description: "A pull request was merged into the repository",
        connection_service: Some("github"),
        fields: PULL_REQUEST_FILTERS,
    },
];

const GITLAB_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "ci_failed",
        label: "Pipeline failed",
        description: "A GitLab CI/CD pipeline completed with a failing status",
        connection_service: Some("gitlab"),
        fields: CI_FILTERS,
    },
    CatalogCapability {
        kind: "ci_succeeded",
        label: "Pipeline succeeded",
        description: "A GitLab CI/CD pipeline completed successfully",
        connection_service: Some("gitlab"),
        fields: CI_FILTERS,
    },
    CatalogCapability {
        kind: "tag_pushed",
        label: "New tag pushed",
        description: "A new Git tag was pushed to the GitLab project",
        connection_service: Some("gitlab"),
        fields: TAG_FILTERS,
    },
];

const OPSWARDEN_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "release_created",
    label: "Release created",
    description: "A Release was created in the Team",
    connection_service: Some("opswarden"),
    fields: RELEASE_FILTERS,
}];

const TIMER_DAILY_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "time",
        label: "Local time",
        description: "Strict 24-hour time in HH:MM format",
        input_type: "time",
        required: true,
        default_value: Some("09:00"),
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "timezone",
        label: "Timezone",
        description: "IANA timezone such as Europe/Paris or UTC",
        input_type: "text",
        required: true,
        default_value: Some("Europe/Paris"),
        options: NO_OPTIONS,
    },
];

const TIMER_INTERVAL_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "minutes",
        label: "Interval in minutes",
        description: "Elapsed minutes between runs, from 5 through 1440",
        input_type: "number",
        required: true,
        default_value: Some("15"),
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "timezone",
        label: "Timezone",
        description: "IANA timezone used to display occurrence context",
        input_type: "text",
        required: true,
        default_value: Some("Europe/Paris"),
        options: NO_OPTIONS,
    },
];

const TIMER_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "daily_at",
        label: "Every day at a local time",
        description: "Run once per local calendar day at the configured time",
        connection_service: Some("timer"),
        fields: TIMER_DAILY_FIELDS,
    },
    CatalogCapability {
        kind: "every_minutes",
        label: "Every number of minutes",
        description: "Run at a bounded elapsed-minute interval",
        connection_service: Some("timer"),
        fields: TIMER_INTERVAL_FIELDS,
    },
];

const GENERIC_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "generic_event",
    label: "Generic JSON event",
    description: "A bounded provider-neutral JSON webhook was received",
    connection_service: Some("generic"),
    fields: GENERIC_FILTERS,
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
        description: "Optional template using normalized event variables such as {{repository}}, {{workflow}}, {{tag}} or {{pull_request_title}}",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const RELEASE_STEP_REACTION_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "release_id",
        label: "Release ID",
        description: "Release UUID or {{release_id}} from the triggering event",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "step",
        label: "Step",
        description: "Exact next step name or an event template",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const BLOCK_RELEASE_REACTION_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "release_id",
        label: "Release ID",
        description: "Release UUID or {{release_id}} from the triggering event",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "severity",
        label: "Blocker severity",
        description: "Severity assigned to the blocking Incident",
        input_type: "select",
        required: true,
        default_value: Some("high"),
        options: SEVERITY_OPTIONS,
    },
    CatalogField {
        name: "title",
        label: "Blocker title",
        description: "Optional Incident title template",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const ESCALATE_INCIDENT_REACTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "incident_id",
    label: "Incident ID",
    description: "Acknowledged Incident UUID or {{incident_id}} from the event",
    input_type: "text",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

const OPSWARDEN_REACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "create_incident",
        label: "Create incident",
        description: "Open an incident in the Team that owns the automation rule",
        connection_service: None,
        fields: INCIDENT_FIELDS,
    },
    CatalogCapability {
        kind: "validate_release_step",
        label: "Validate Release step",
        description: "Validate the next sequential step of a Release",
        connection_service: None,
        fields: RELEASE_STEP_REACTION_FIELDS,
    },
    CatalogCapability {
        kind: "block_release",
        label: "Block Release",
        description: "Create and link an active blocker Incident to an in-progress Release",
        connection_service: None,
        fields: BLOCK_RELEASE_REACTION_FIELDS,
    },
    CatalogCapability {
        kind: "escalate_incident",
        label: "Escalate Incident",
        description: "Escalate an acknowledged Incident while preserving its lifecycle",
        connection_service: None,
        fields: ESCALATE_INCIDENT_REACTION_FIELDS,
    },
];

const HTTP_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "http_notify",
    label: "Send HTTP notification",
    description: "Send a notification through a configured HTTP connection",
    connection_service: Some("http"),
    fields: &[CatalogField {
        name: "message",
        label: "Message",
        description: "Template using normalized event variables such as {{repository}}, {{workflow}}, {{tag}} or {{pull_request_title}}",
        input_type: "text",
        required: false,
        default_value: Some("Automation event on {{repository}}"),
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

const GITLAB_CONNECTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "webhook_signing_secret",
    label: "Webhook secret token",
    description: "Required on first connection; sent by GitLab in X-Gitlab-Token",
    input_type: "password",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

const GENERIC_CONNECTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "webhook_signing_secret",
    label: "Shared webhook token",
    description: "Required on first connection; sent in X-OpsWarden-Token",
    input_type: "password",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

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
        service: "gitlab",
        label: "GitLab",
        actions: GITLAB_ACTIONS,
        reactions: &[],
        connection: Some(CatalogConnection {
            description: "Verify incoming GitLab webhooks with their secret token",
            fields: GITLAB_CONNECTION_FIELDS,
            oauth: None,
            testable: false,
        }),
    },
    AutomationServiceDefinition {
        service: "generic",
        label: "Generic Webhook",
        actions: GENERIC_ACTIONS,
        reactions: &[],
        connection: Some(CatalogConnection {
            description: "Receive bounded JSON webhooks authenticated with a shared token",
            fields: GENERIC_CONNECTION_FIELDS,
            oauth: None,
            testable: false,
        }),
    },
    AutomationServiceDefinition {
        service: "opswarden",
        label: "OpsWarden",
        actions: OPSWARDEN_ACTIONS,
        reactions: OPSWARDEN_REACTIONS,
        connection: None,
    },
    AutomationServiceDefinition {
        service: "timer",
        label: "Timer",
        actions: TIMER_ACTIONS,
        reactions: &[],
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
        assert!(supports_action("github", "ci_succeeded"));
        assert!(supports_action("github", "tag_pushed"));
        assert!(supports_action("github", "pr_merged"));
        assert!(supports_action("gitlab", "ci_failed"));
        assert!(supports_action("gitlab", "ci_succeeded"));
        assert!(supports_action("gitlab", "tag_pushed"));
        assert!(supports_action("generic", "generic_event"));
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
}
