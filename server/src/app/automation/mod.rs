// --- server/src/app/automation/mod.rs ---

pub mod ingest_webhook;

pub use ingest_webhook::{IngestWebhookCommand, IngestWebhookResult, IngestWebhookUseCase};
