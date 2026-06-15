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

use crate::app::automation::{IngestWebhookCommand, IngestWebhookUseCase};
use crate::domain::error::DomainError;
use crate::AppState;

#[derive(Serialize)]
pub struct WebhookReceipt {
    pub received: bool,
    pub rules_triggered: usize,
}

/// `POST /webhooks/{service}` — public, authenticated by the request's HMAC
/// signature (the shared secret in the vault), never by a JWT.
pub async fn receive(
    State(state): State<AppState>,
    Path(service): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<WebhookReceipt>), DomainError> {
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let use_case = IngestWebhookUseCase::new(
        state.vault.clone(),
        state.webhook_verifier.clone(),
        state.webhook_parser.clone(),
        state.rules.clone(),
        state.incidents.clone(),
        state.notifier.clone(),
        state.events.clone(),
    );

    let result = use_case
        .ingest(IngestWebhookCommand {
            service,
            signature,
            body: body.to_vec(),
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(WebhookReceipt {
            received: true,
            rules_triggered: result.rules_triggered,
        }),
    ))
}
