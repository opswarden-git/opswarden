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

use crate::adapters::metrics::AlertmanagerOutcome;
use crate::adapters::webhook::{alertmanager, generic::validate_payload};
use crate::app::automation::IngestTeamWebhookCommand;
use crate::domain::automation_catalog::{
    ALERTMANAGER_SERVICE, GENERIC_SERVICE, GITHUB_SERVICE, GITLAB_SERVICE,
};
use crate::domain::error::DomainError;
use crate::AppState;

#[derive(Serialize)]
pub struct TeamWebhookReceipt {
    pub received: bool,
    pub duplicate: bool,
    pub transitions_received: usize,
    pub transitions_duplicate: usize,
    pub transitions_ignored: usize,
    pub rules_triggered: usize,
    pub rules_failed: usize,
}

impl TeamWebhookReceipt {
    fn single(result: crate::app::automation::IngestTeamWebhookResult) -> Self {
        Self {
            received: true,
            duplicate: result.duplicate,
            transitions_received: 1,
            transitions_duplicate: usize::from(result.duplicate),
            transitions_ignored: usize::from(result.ignored),
            rules_triggered: result.rules_triggered,
            rules_failed: result.rules_failed,
        }
    }
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

    let result = state
        .webhook_ingress
        .accept(IngestTeamWebhookCommand {
            connection_id,
            expected_service: GITHUB_SERVICE,
            provider_delivery_id,
            provider_event,
            signature,
            body: body.to_vec(),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(TeamWebhookReceipt::single(result)),
    ))
}

pub async fn receive_gitlab_for_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<TeamWebhookReceipt>), DomainError> {
    let provider_delivery_id = required_header(&headers, "X-Gitlab-Event-UUID")?;

    let provider_event = required_header(&headers, "X-Gitlab-Event")?;
    let signature = headers
        .get("X-Gitlab-Token")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let result = state
        .webhook_ingress
        .accept(IngestTeamWebhookCommand {
            connection_id,
            expected_service: GITLAB_SERVICE,
            provider_delivery_id,
            provider_event,
            signature,
            body: body.to_vec(),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(TeamWebhookReceipt::single(result)),
    ))
}

/// Provider-neutral JSON webhook. Authentication, idempotency and event type
/// are explicit headers; the payload is validated before durable ingestion.
pub async fn receive_generic_for_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<TeamWebhookReceipt>), DomainError> {
    if !is_json_content_type(&headers) {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    let provider_delivery_id = required_header(&headers, "X-OpsWarden-Delivery")?;
    let provider_event = required_header(&headers, "X-OpsWarden-Event")?;
    let signature = Some(required_header(&headers, "X-OpsWarden-Token")?);
    validate_payload(&body)?;

    let result = state
        .webhook_ingress
        .accept(IngestTeamWebhookCommand {
            connection_id,
            expected_service: GENERIC_SERVICE,
            provider_delivery_id,
            provider_event,
            signature,
            body: body.to_vec(),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(TeamWebhookReceipt::single(result)),
    ))
}

pub async fn receive_alertmanager_for_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<TeamWebhookReceipt>), DomainError> {
    let metrics = state.alertmanager_metrics.clone();
    let response = receive_alertmanager(state, connection_id, headers, body).await;
    if let Err(error) = &response {
        metrics.record(error_outcome(error));
    }
    response
}

async fn receive_alertmanager(
    state: AppState,
    connection_id: uuid::Uuid,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<TeamWebhookReceipt>), DomainError> {
    if !is_json_content_type(&headers) {
        return Err(DomainError::InvalidWebhookDelivery);
    }
    let transitions = alertmanager::transitions(&body)?;
    let signature = Some(bearer_token(&headers)?);
    let mut receipt = TeamWebhookReceipt {
        received: true,
        duplicate: true,
        transitions_received: transitions.len(),
        transitions_duplicate: 0,
        transitions_ignored: 0,
        rules_triggered: 0,
        rules_failed: 0,
    };
    let commands = transitions
        .into_iter()
        .map(|transition| IngestTeamWebhookCommand {
            connection_id,
            expected_service: ALERTMANAGER_SERVICE,
            provider_delivery_id: transition.delivery_id,
            provider_event: "alertmanager_webhook".to_string(),
            signature: signature.clone(),
            body: transition.body,
        })
        .collect();
    let results = state.webhook_ingress.accept_batch(commands).await?;
    for result in results {
        state.alertmanager_metrics.record(result_outcome(&result));
        receipt.transitions_duplicate += usize::from(result.duplicate);
        receipt.transitions_ignored += usize::from(result.ignored);
        receipt.rules_triggered += result.rules_triggered;
        receipt.rules_failed += result.rules_failed;
    }
    receipt.duplicate = receipt.transitions_duplicate == receipt.transitions_received;

    Ok((StatusCode::ACCEPTED, Json(receipt)))
}

fn result_outcome(result: &crate::app::automation::IngestTeamWebhookResult) -> AlertmanagerOutcome {
    if result.rules_failed > 0 {
        AlertmanagerOutcome::Failed
    } else if result.duplicate {
        AlertmanagerOutcome::Duplicate
    } else if result.ignored {
        AlertmanagerOutcome::Ignored
    } else {
        AlertmanagerOutcome::Accepted
    }
}

fn error_outcome(error: &DomainError) -> AlertmanagerOutcome {
    match error {
        DomainError::Storage | DomainError::Crypto | DomainError::InvalidAutomationTransition => {
            AlertmanagerOutcome::Failed
        }
        _ => AlertmanagerOutcome::Rejected,
    }
}

fn bearer_token(headers: &HeaderMap) -> Result<String, DomainError> {
    let authorization = required_header(headers, "Authorization")?;
    let token = authorization
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or(DomainError::InvalidWebhookDelivery)?;
    Ok(token.to_string())
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

fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}
