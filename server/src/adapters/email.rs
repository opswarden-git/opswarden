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

use std::net::IpAddr;

/// Validates that an SMTP host is not a private, loopback, or link-local network destination.
fn validate_smtp_host(host: &str) -> Result<(), DomainError> {
    let trimmed = host.trim().to_lowercase();
    if trimmed.is_empty()
        || trimmed == "localhost"
        || trimmed.ends_with(".local")
        || trimmed.ends_with(".localhost")
        || trimmed.ends_with(".internal")
    {
        error!("SMTP host {} is a local or internal destination", host);
        return Err(DomainError::EmailTransportError);
    }

    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_loopback()
                    || ipv4.is_private()
                    || ipv4.is_link_local()
                    || ipv4.is_unspecified()
                    || ipv4.is_broadcast()
                {
                    error!("SMTP IP address {} is private or loopback", ip);
                    return Err(DomainError::EmailTransportError);
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_loopback() || ipv6.is_unspecified() {
                    error!("SMTP IPv6 address {} is private or loopback", ip);
                    return Err(DomainError::EmailTransportError);
                }
            }
        }
    }

    Ok(())
}

impl SmtpEmailSender {
    pub fn new() -> Self {
        Self
    }

    /// Build an authenticated, timeout-bounded encrypted transport. SMTPS ports
    /// 465 and 2465 use implicit TLS; submission ports such as 587 require
    /// STARTTLS. Shared by the connection test and the reaction so both exercise
    /// the same handshake.
    fn transport(config: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
        validate_smtp_host(&config.host)?;
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
            host: "smtp.example.com".to_string(),
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

    #[test]
    fn ssrf_hosts_are_rejected() {
        assert_eq!(
            validate_smtp_host("localhost"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("127.0.0.1"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("10.0.0.1"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("192.168.1.1"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("169.254.169.254"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("server.local"),
            Err(DomainError::EmailTransportError)
        );
        assert_eq!(
            validate_smtp_host("internal.localhost"),
            Err(DomainError::EmailTransportError)
        );
        assert!(validate_smtp_host("smtp.example.com").is_ok());
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
