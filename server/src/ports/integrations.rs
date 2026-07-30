use async_trait::async_trait;

use crate::domain::error::DomainError;

/// Outbound notification REAction: POST a `message` to a `url`. One generic
/// connector — a Slack incoming webhook, Discord, Teams or any HTTP endpoint is
/// just a URL. The transport (reqwest, payload shape) is an adapter concern.
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Validate syntax, DNS and the resolved network target without sending a
    /// business notification. Configuration and execution share this boundary.
    async fn validate_endpoint(&self, url: &str) -> Result<(), DomainError>;
    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError>;
}

/// SMTP coordinates and sender identity held by one Team's email connection.
/// Grouping them keeps five interchangeable `&str` arguments from being silently
/// swapped at a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

/// A bounded outbound message. Templates are interpolated by the use-case, so the
/// adapter only transports what it is handed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Outbound email REAction. Resolves an external SMTP server, validates the target
/// address, and transmits a bounded notification.
#[async_trait]
pub trait EmailSender: Send + Sync {
    /// Open an authenticated SMTP session and issue NOOP without delivering a
    /// business message. Configuration and execution share this boundary, the way
    /// `Notifier::validate_endpoint` mirrors `Notifier::notify`.
    async fn validate_smtp(&self, config: &SmtpConfig) -> Result<(), DomainError>;

    async fn send_email(
        &self,
        config: &SmtpConfig,
        message: &EmailMessage,
    ) -> Result<(), DomainError>;
}

/// A normalized GIF search result, independent of the provider's JSON shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GifResult {
    pub id: String,
    pub title: String,
    /// URL of the animated GIF to display when selected.
    pub url: String,
    /// Smaller still/preview URL for the results grid.
    pub preview_url: String,
    pub width: u32,
    pub height: u32,
}

/// External GIF search backed by GIPHY. The
/// provider, HTTP transport and API-key handling are adapter concerns; the
/// use-case only ever sees normalized `GifResult`s.
#[async_trait]
pub trait GifSearch: Send + Sync {
    async fn search(
        &self,
        query: &str,
        limit: u32,
        rating: &str,
    ) -> Result<Vec<GifResult>, DomainError>;
}
