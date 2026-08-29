use super::{
    CatalogCapability, CatalogField, ReactionExecutor, GENERIC_SERVICE, GITHUB_SERVICE,
    GITLAB_SERVICE, HTTP_SERVICE, OPSWARDEN_SERVICE, TIMER_SERVICE,
};

pub(super) const NO_OPTIONS: &[&str] = &[];
pub(super) const SEVERITY_OPTIONS: &[&str] = &["low", "medium", "high", "critical"];
pub(super) const CI_FILTERS: &[CatalogField] = &[
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

pub(super) const TAG_FILTERS: &[CatalogField] = &[
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

pub(super) const PULL_REQUEST_FILTERS: &[CatalogField] = &[
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

pub(super) const RELEASE_FILTERS: &[CatalogField] = &[
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

pub(super) const GENERIC_FILTERS: &[CatalogField] = &[
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

pub(super) const GITHUB_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "ci_failed",
        label: "CI run failed",
        description: "A GitHub Actions workflow run completed with a failing conclusion",
        connection_service: Some(GITHUB_SERVICE),
        fields: CI_FILTERS,
        executor: None,
    },
    CatalogCapability {
        kind: "ci_succeeded",
        label: "CI run succeeded",
        description: "A GitHub Actions workflow run completed successfully",
        connection_service: Some(GITHUB_SERVICE),
        fields: CI_FILTERS,
        executor: None,
    },
    CatalogCapability {
        kind: "tag_pushed",
        label: "New tag pushed",
        description: "A new Git tag was pushed to the repository",
        connection_service: Some(GITHUB_SERVICE),
        fields: TAG_FILTERS,
        executor: None,
    },
    CatalogCapability {
        kind: "pr_merged",
        label: "Pull request merged",
        description: "A pull request was merged into the repository",
        connection_service: Some(GITHUB_SERVICE),
        fields: PULL_REQUEST_FILTERS,
        executor: None,
    },
];

pub(super) const GITLAB_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "ci_failed",
        label: "Pipeline failed",
        description: "A GitLab CI/CD pipeline completed with a failing status",
        connection_service: Some(GITLAB_SERVICE),
        fields: CI_FILTERS,
        executor: None,
    },
    CatalogCapability {
        kind: "ci_succeeded",
        label: "Pipeline succeeded",
        description: "A GitLab CI/CD pipeline completed successfully",
        connection_service: Some(GITLAB_SERVICE),
        fields: CI_FILTERS,
        executor: None,
    },
    CatalogCapability {
        kind: "tag_pushed",
        label: "New tag pushed",
        description: "A new Git tag was pushed to the GitLab project",
        connection_service: Some(GITLAB_SERVICE),
        fields: TAG_FILTERS,
        executor: None,
    },
];

pub(super) const OPSWARDEN_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "release_created",
    label: "Release created",
    description: "A Release was created in the Team",
    connection_service: Some(OPSWARDEN_SERVICE),
    fields: RELEASE_FILTERS,
    executor: None,
}];

pub(super) const TIMER_DAILY_FIELDS: &[CatalogField] = &[
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

pub(super) const TIMER_INTERVAL_FIELDS: &[CatalogField] = &[
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

pub(super) const TIMER_ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "daily_at",
        label: "Every day at a local time",
        description: "Run once per local calendar day at the configured time",
        connection_service: Some(TIMER_SERVICE),
        fields: TIMER_DAILY_FIELDS,
        executor: None,
    },
    CatalogCapability {
        kind: "every_minutes",
        label: "Every number of minutes",
        description: "Run at a bounded elapsed-minute interval",
        connection_service: Some(TIMER_SERVICE),
        fields: TIMER_INTERVAL_FIELDS,
        executor: None,
    },
];

pub(super) const GENERIC_ACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "generic_event",
    label: "Generic JSON event",
    description: "A bounded provider-neutral JSON webhook was received",
    connection_service: Some(GENERIC_SERVICE),
    fields: GENERIC_FILTERS,
    executor: None,
}];

pub(super) const INCIDENT_FIELDS: &[CatalogField] = &[
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

pub(super) const RELEASE_STEP_REACTION_FIELDS: &[CatalogField] = &[
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

pub(super) const BLOCK_RELEASE_REACTION_FIELDS: &[CatalogField] = &[
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

pub(super) const ESCALATE_INCIDENT_REACTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "incident_id",
    label: "Incident ID",
    description: "Acknowledged Incident UUID or {{incident_id}} from the event",
    input_type: "text",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

pub(super) const OPSWARDEN_REACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "create_incident",
        label: "Create incident",
        description: "Open an incident in the Team that owns the automation rule",
        connection_service: None,
        fields: INCIDENT_FIELDS,
        executor: Some(ReactionExecutor::CreateIncident),
    },
    CatalogCapability {
        kind: "validate_release_step",
        label: "Validate Release step",
        description: "Validate the next sequential step of a Release",
        connection_service: None,
        fields: RELEASE_STEP_REACTION_FIELDS,
        executor: Some(ReactionExecutor::ValidateReleaseStep),
    },
    CatalogCapability {
        kind: "block_release",
        label: "Block Release",
        description: "Create and link an active blocker Incident to an in-progress Release",
        connection_service: None,
        fields: BLOCK_RELEASE_REACTION_FIELDS,
        executor: Some(ReactionExecutor::BlockRelease),
    },
    CatalogCapability {
        kind: "escalate_incident",
        label: "Escalate Incident",
        description: "Escalate an acknowledged Incident while preserving its lifecycle",
        connection_service: None,
        fields: ESCALATE_INCIDENT_REACTION_FIELDS,
        executor: Some(ReactionExecutor::EscalateIncident),
    },
];

pub(super) const HTTP_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "http_notify",
    label: "Send HTTP notification",
    description: "Send a notification through a configured HTTP connection",
    connection_service: Some(HTTP_SERVICE),
    fields: &[CatalogField {
        name: "message",
        label: "Message",
        description: "Template using normalized event variables such as {{repository}}, {{workflow}}, {{tag}} or {{pull_request_title}}",
        input_type: "text",
        required: false,
        default_value: Some("Automation event on {{repository}}"),
        options: NO_OPTIONS,
    }],
    executor: Some(ReactionExecutor::HttpNotify),
}];
