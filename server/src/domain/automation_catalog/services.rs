use super::alertmanager;
use super::capabilities::*;
use super::{
    AutomationServiceDefinition, CatalogCapability, CatalogConnection, CatalogField, CatalogOAuth,
    ConnectionConfigurator, ConnectionProbe, ReactionExecutor, WebhookAuthentication,
    EMAIL_SERVICE, GENERIC_SERVICE, GITHUB_SERVICE, GITLAB_SERVICE, HTTP_SERVICE,
    OPSWARDEN_SERVICE, TIMER_SERVICE,
};

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

const EMAIL_CONNECTION_FIELDS: &[CatalogField] = &[
    CatalogField {
        name: "smtp_host",
        label: "SMTP Host",
        description: "Hostname or IP address of the SMTP server",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "smtp_port",
        label: "SMTP Port",
        description: "Port number (usually 587 or 465)",
        input_type: "number",
        required: true,
        default_value: Some("587"),
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "smtp_username",
        label: "SMTP Username",
        description: "Username for SMTP authentication",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "smtp_password",
        label: "SMTP Password",
        description: "Password for SMTP authentication",
        input_type: "password",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "from_address",
        label: "From Address",
        description: "The sender email address",
        input_type: "text",
        required: true,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const EMAIL_REACTIONS: &[CatalogCapability] = &[CatalogCapability {
    kind: "email_notify",
    label: "Send email",
    description: "Send an email to a configured recipient",
    connection_service: Some(EMAIL_SERVICE),
    fields: &[
        CatalogField {
            name: "to",
            label: "Recipient (To)",
            description: "The destination email address",
            input_type: "text",
            required: true,
            default_value: None,
            options: NO_OPTIONS,
        },
        CatalogField {
            name: "subject",
            label: "Subject",
            description: "Template using normalized event variables such as {{repository}}, {{workflow}}, {{tag}} or {{pull_request_title}}",
            input_type: "text",
            required: false,
            default_value: Some("Automation event on {{repository}}"),
            options: NO_OPTIONS,
        },
        CatalogField {
            name: "body",
            label: "Body",
            description: "Template using normalized event variables such as {{repository}}, {{workflow}}, {{tag}} or {{pull_request_title}}",
            input_type: "text",
            required: false,
            default_value: Some("Automation event on {{repository}}"),
            options: NO_OPTIONS,
        },
    ],
    executor: Some(ReactionExecutor::EmailNotify),
}];

pub const AUTOMATION_CATALOG: &[AutomationServiceDefinition] = &[
    AutomationServiceDefinition {
        service: GITHUB_SERVICE,
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
            configurator: ConnectionConfigurator::Github,
            webhook_authentication: Some(WebhookAuthentication::Signature),
            probe: None,
        }),
        internal: false,
    },
    AutomationServiceDefinition {
        service: GITLAB_SERVICE,
        label: "GitLab",
        actions: GITLAB_ACTIONS,
        reactions: &[],
        connection: Some(CatalogConnection {
            description: "Verify incoming GitLab webhooks with their secret token",
            fields: GITLAB_CONNECTION_FIELDS,
            oauth: None,
            configurator: ConnectionConfigurator::TokenWebhook,
            webhook_authentication: Some(WebhookAuthentication::Token),
            probe: None,
        }),
        internal: false,
    },
    AutomationServiceDefinition {
        service: GENERIC_SERVICE,
        label: "Generic Webhook",
        actions: GENERIC_ACTIONS,
        reactions: &[],
        connection: Some(CatalogConnection {
            description: "Receive bounded JSON webhooks authenticated with a shared token",
            fields: GENERIC_CONNECTION_FIELDS,
            oauth: None,
            configurator: ConnectionConfigurator::TokenWebhook,
            webhook_authentication: Some(WebhookAuthentication::Token),
            probe: None,
        }),
        internal: false,
    },
    alertmanager::DEFINITION,
    AutomationServiceDefinition {
        service: OPSWARDEN_SERVICE,
        label: "OpsWarden",
        actions: OPSWARDEN_ACTIONS,
        reactions: OPSWARDEN_REACTIONS,
        connection: None,
        internal: true,
    },
    AutomationServiceDefinition {
        service: TIMER_SERVICE,
        label: "Timer",
        actions: TIMER_ACTIONS,
        reactions: &[],
        connection: None,
        internal: true,
    },
    AutomationServiceDefinition {
        service: HTTP_SERVICE,
        label: "HTTP",
        actions: &[],
        reactions: HTTP_REACTIONS,
        connection: Some(CatalogConnection {
            description: "Send bounded notifications to a public HTTPS endpoint",
            fields: HTTP_CONNECTION_FIELDS,
            oauth: None,
            configurator: ConnectionConfigurator::Http,
            webhook_authentication: None,
            probe: Some(ConnectionProbe::Http),
        }),
        internal: false,
    },
    AutomationServiceDefinition {
        service: EMAIL_SERVICE,
        label: "Email",
        actions: &[],
        reactions: EMAIL_REACTIONS,
        connection: Some(CatalogConnection {
            description: "Send emails via an external SMTP server",
            fields: EMAIL_CONNECTION_FIELDS,
            oauth: None,
            configurator: ConnectionConfigurator::Email,
            webhook_authentication: None,
            probe: Some(ConnectionProbe::Email),
        }),
        internal: false,
    },
];
