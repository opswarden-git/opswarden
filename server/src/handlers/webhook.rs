// --- server/src/handlers/webhook.rs ---
//
// Inbound webhook transport. No business logic here: read the raw body and the
// signature header, hand them to the use-case, map the outcome to HTTP. The body
// is taken as raw `Bytes` so the HMAC is checked against the exact bytes received
// (re-serializing JSON would change them and break the signature).

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::app::automation::{
    IngestTeamWebhookCommand, IngestTeamWebhookUseCase, TeamWebhookDependencies,
};
use crate::domain::error::DomainError;
use crate::AppState;

#[derive(Serialize)]
pub struct TeamWebhookReceipt {
    pub received: bool,
    pub duplicate: bool,
    pub rules_triggered: usize,
    pub rules_failed: usize,
}

/// `POST /webhooks/github/{connection_id}` — durable R9 webhook endpoint.
/// GitHub's delivery id is the idempotency key; all provider headers are read
/// before the raw bytes are passed unchanged to HMAC verification.
pub async fn receive_github_for_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<TeamWebhookReceipt>), DomainError> {
    let provider_delivery_id = required_header(&headers, "X-GitHub-Delivery")?;
    let provider_event = required_header(&headers, "X-GitHub-Event")?;
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let result = IngestTeamWebhookUseCase::new(TeamWebhookDependencies {
        connections: state.service_connections.clone(),
        credentials: state.connection_credentials.clone(),
        verifier: state.webhook_verifier.clone(),
        parser: state.webhook_parser.clone(),
        deliveries: state.webhook_deliveries.clone(),
        rules: state.automation_rules.clone(),
        runs: state.automation_runs.clone(),
        incidents: state.incidents.clone(),
        notifier: state.notifier.clone(),
        events: state.events.clone(),
    })
    .ingest(IngestTeamWebhookCommand {
        connection_id,
        provider_delivery_id,
        provider_event,
        signature,
        body: body.to_vec(),
    })
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(TeamWebhookReceipt {
            received: true,
            duplicate: result.duplicate,
            rules_triggered: result.rules_triggered,
            rules_failed: result.rules_failed,
        }),
    ))
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, DomainError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(DomainError::InvalidWebhookDelivery)
}
