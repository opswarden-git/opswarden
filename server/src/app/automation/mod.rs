// --- server/src/app/automation/mod.rs ---

pub mod ingest_webhook;
pub mod service_connection;

pub use ingest_webhook::{IngestWebhookCommand, IngestWebhookResult, IngestWebhookUseCase};
pub use service_connection::ServiceConnectionUseCase;
