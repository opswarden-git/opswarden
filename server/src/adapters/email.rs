use crate::domain::error::DomainError;
use crate::ports::{EmailMessage, EmailSender, SmtpConfig};
use async_trait::async_trait;
use lettre::message::header;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::time::Duration;
use tracing::{error, info};

/// Bounds every SMTP exchange so a slow or black-holed relay cannot pin an
/// automation run open.
const SMTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
pub struct SmtpEmailSender;

impl SmtpEmailSender {
    pub fn new() -> Self {
        Self
    }

    /// Build an authenticated, timeout-bounded encrypted transport. SMTPS ports
    /// 465 and 2465 use implicit TLS; submission ports such as 587 require
    /// STARTTLS. Shared by the connection test and the reaction so both exercise
    /// the same handshake.
    fn transport(config: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
        let credentials = Credentials::new(config.username.clone(), config.password.clone());
        let builder = if matches!(config.port, 465 | 2465) {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
        }
        .map_err(|error| {
            error!("Invalid SMTP host {}: {}", config.host, error);
            DomainError::EmailTransportError
        })?;
        let transport = builder
            .port(config.port)
            .credentials(credentials)
            .timeout(Some(SMTP_TIMEOUT))
            .build();
        Ok(transport)
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn validate_smtp(&self, config: &SmtpConfig) -> Result<(), DomainError> {
        // Reject a malformed sender identity before opening a socket: a Manager
        // fixing a typo should not wait on a network round-trip.
        config
            .from
            .parse::<lettre::Address>()
            .map_err(|_| DomainError::InvalidEmailSender)?;

        let transport = Self::transport(config)?;
        match transport.test_connection().await {
            Ok(true) => Ok(()),
            Ok(false) => {
                error!("SMTP server {} refused the NOOP probe", config.host);
                Err(DomainError::EmailTransportError)
            }
            Err(error) => {
                error!("SMTP connection test failed: {}", error);
                Err(DomainError::EmailTransportError)
            }
        }
    }

    async fn send_email(
        &self,
        config: &SmtpConfig,
        message: &EmailMessage,
    ) -> Result<(), DomainError> {
        let from: lettre::Address = config
            .from
            .parse()
            .map_err(|_| DomainError::InvalidEmailSender)?;
        let to: lettre::Address = message
            .to
            .parse()
            .map_err(|_| DomainError::InvalidEmailRecipient)?;

        let email = Message::builder()
            .from(lettre::message::Mailbox::new(None, from))
            .to(lettre::message::Mailbox::new(None, to))
            .subject(&message.subject)
            .header(header::ContentType::TEXT_PLAIN)
            .body(message.body.clone())
            .map_err(|error| {
                error!("Failed to build email message: {}", error);
                DomainError::EmailTransportError
            })?;

        let transport = Self::transport(config)?;
        info!("Sending email via SMTP to {}", message.to);
        match transport.send(email).await {
            Ok(_) => Ok(()),
            Err(error) => {
                error!("SMTP transport error: {}", error);
                Err(DomainError::EmailTransportError)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SmtpConfig {
        SmtpConfig {
            host: "localhost".to_string(),
            port: 587,
            username: "user".to_string(),
            password: "pass".to_string(),
            from: "from@example.com".to_string(),
        }
    }

    fn message(to: &str) -> EmailMessage {
        EmailMessage {
            to: to.to_string(),
            subject: "Subj".to_string(),
            body: "Body".to_string(),
        }
    }

    #[tokio::test]
    async fn invalid_to_address_returns_error() {
        let error = SmtpEmailSender::new()
            .send_email(&config(), &message("invalid-email"))
            .await
            .unwrap_err();
        assert_eq!(error, DomainError::InvalidEmailRecipient);
    }

    #[tokio::test]
    async fn invalid_from_address_is_reported_as_a_sender_error() {
        let mut config = config();
        config.from = "not-an-address".to_string();
        let error = SmtpEmailSender::new()
            .send_email(&config, &message("to@example.com"))
            .await
            .unwrap_err();
        assert_eq!(error, DomainError::InvalidEmailSender);
    }

    #[tokio::test]
    async fn validating_a_malformed_sender_never_opens_a_connection() {
        let mut config = config();
        config.from = "not-an-address".to_string();
        // Unroutable host: reaching the network would surface EmailTransportError
        // instead, so this also proves the check happens before dialing.
        config.host = "smtp.invalid".to_string();
        let error = SmtpEmailSender::new()
            .validate_smtp(&config)
            .await
            .unwrap_err();
        assert_eq!(error, DomainError::InvalidEmailSender);
    }
}
