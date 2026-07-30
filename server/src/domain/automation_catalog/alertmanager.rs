use super::capabilities::{NO_OPTIONS, SEVERITY_OPTIONS};
use super::{AutomationServiceDefinition, CatalogCapability, CatalogConnection, CatalogField};

const FILTERS: &[CatalogField] = &[
    CatalogField {
        name: "severity",
        label: "Severity",
        description: "Only match this alert severity",
        input_type: "select",
        required: false,
        default_value: None,
        options: SEVERITY_OPTIONS,
    },
    CatalogField {
        name: "alertname",
        label: "Alert name",
        description: "Only match this alert name",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
    CatalogField {
        name: "receiver",
        label: "Receiver",
        description: "Only match this Alertmanager receiver",
        input_type: "text",
        required: false,
        default_value: None,
        options: NO_OPTIONS,
    },
];

const ACTIONS: &[CatalogCapability] = &[
    CatalogCapability {
        kind: "alert_firing",
        label: "Alert firing",
        description: "One authenticated Alertmanager alert entered the firing state",
        connection_service: Some("alertmanager"),
        fields: FILTERS,
    },
    CatalogCapability {
        kind: "alert_resolved",
        label: "Alert resolved",
        description: "One authenticated Alertmanager alert entered the resolved state",
        connection_service: Some("alertmanager"),
        fields: FILTERS,
    },
];

const CONNECTION_FIELDS: &[CatalogField] = &[CatalogField {
    name: "webhook_signing_secret",
    label: "Bearer token",
    description: "Required on first connection; sent as Authorization: Bearer <token>",
    input_type: "password",
    required: true,
    default_value: None,
    options: NO_OPTIONS,
}];

pub(super) const DEFINITION: AutomationServiceDefinition = AutomationServiceDefinition {
    service: "alertmanager",
    label: "Alertmanager",
    actions: ACTIONS,
    reactions: &[],
    connection: Some(CatalogConnection {
        description: "Receive Alertmanager notification groups authenticated with a bearer token",
        fields: CONNECTION_FIELDS,
        oauth: None,
        testable: false,
    }),
};
