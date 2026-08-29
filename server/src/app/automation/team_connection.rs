use std::sync::Arc;

use uuid::Uuid;

use super::reaction_executor::smtp_config;
use super::team_access::require_manager;
use crate::domain::automation_catalog::{
    service, ConnectionConfigurator, ConnectionProbe, EMAIL_SERVICE, GITHUB_SERVICE, HTTP_SERVICE,
};
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::domain::user::Email;
use crate::ports::{
    ConnectionCredentialVault, EmailSender, Notifier, ServiceConnectionRepo, TeamRepo,
};

const CONNECTION_TEST_MESSAGE: &str = "OpsWarden connection test";

pub struct ConfigureGithubConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub webhook_signing_secret: Option<String>,
    pub personal_token: Option<String>,
}

pub struct ConfigureTokenWebhookConnectionCommand {
    pub service: &'static str,
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub webhook_token: Option<String>,
}

pub struct ConfigureHttpConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub endpoint_url: String,
}

/// Every field is optional so a Manager can rotate one credential without
/// retyping the others; the use-case still requires a complete set the first
/// time the connection is created.
pub struct ConfigureEmailConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub from_address: Option<String>,
}

pub struct TestConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub connection_id: Uuid,
}

pub struct ListTeamConnectionsCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct DeleteTeamConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub connection_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamConnectionView {
    pub connection: ServiceConnection,
    pub configured_credentials: Vec<CredentialKind>,
}

pub struct TeamConnectionUseCase {
    teams: Arc<dyn TeamRepo>,
    connections: Arc<dyn ServiceConnectionRepo>,
    credentials: Arc<dyn ConnectionCredentialVault>,
    notifier: Arc<dyn Notifier>,
    email_sender: Arc<dyn EmailSender>,
}

impl TeamConnectionUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        connections: Arc<dyn ServiceConnectionRepo>,
        credentials: Arc<dyn ConnectionCredentialVault>,
        notifier: Arc<dyn Notifier>,
        email_sender: Arc<dyn EmailSender>,
    ) -> Self {
        Self {
            teams,
            connections,
            credentials,
            notifier,
            email_sender,
        }
    }

    pub async fn configure_github(
        &self,
        cmd: ConfigureGithubConnectionCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        validate_optional_secret(&cmd.webhook_signing_secret)?;
        validate_optional_secret(&cmd.personal_token)?;
        if cmd.webhook_signing_secret.is_none() && cmd.personal_token.is_none() {
            return Err(DomainError::InvalidServiceSecret);
        }

        let existing = self
            .connections
            .find_connection_by_service(cmd.team_id, GITHUB_SERVICE)
            .await?;
        if existing.is_none() && cmd.webhook_signing_secret.is_none() {
            return Err(DomainError::InvalidServiceSecret);
        }

        let connection = match existing {
            Some(connection) => connection,
            None => {
                let connection =
                    ServiceConnection::new(cmd.team_id, GITHUB_SERVICE, cmd.requester_id)?;
                self.connections.insert_connection(&connection).await?;
                connection
            }
        };

        let signing_secret_replaced = cmd.webhook_signing_secret.is_some();
        if let Some(secret) = cmd.webhook_signing_secret {
            self.credentials
                .store_credential(connection.id, CredentialKind::WebhookSigningSecret, &secret)
                .await?;
        }
        if let Some(token) = cmd.personal_token {
            self.credentials
                .store_credential(connection.id, CredentialKind::PersonalToken, &token)
                .await?;
        }
        if signing_secret_replaced {
            self.connections
                .reset_connection_health(connection.id)
                .await?;
        }

        self.connection_view(cmd.team_id, connection.id).await
    }

    pub async fn configure_token_webhook(
        &self,
        cmd: ConfigureTokenWebhookConnectionCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        let definition = service(cmd.service).ok_or(DomainError::InvalidServiceConnection)?;
        if definition
            .connection
            .map(|connection| connection.configurator)
            != Some(ConnectionConfigurator::TokenWebhook)
        {
            return Err(DomainError::InvalidServiceConnection);
        }
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        validate_optional_secret(&cmd.webhook_token)?;
        let existing = self
            .connections
            .find_connection_by_service(cmd.team_id, definition.service)
            .await?;
        if existing.is_none() && cmd.webhook_token.is_none() {
            return Err(DomainError::InvalidServiceSecret);
        }
        let connection = match existing {
            Some(connection) => connection,
            None => {
                let connection =
                    ServiceConnection::new(cmd.team_id, definition.service, cmd.requester_id)?;
                self.connections.insert_connection(&connection).await?;
                connection
            }
        };
        if let Some(token) = cmd.webhook_token {
            self.credentials
                .store_credential(connection.id, CredentialKind::WebhookSigningSecret, &token)
                .await?;
            self.connections
                .reset_connection_health(connection.id)
                .await?;
        }
        self.connection_view(cmd.team_id, connection.id).await
    }

    pub async fn configure_http(
        &self,
        cmd: ConfigureHttpConnectionCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        if cmd.endpoint_url.trim().is_empty() {
            return Err(DomainError::InvalidReactionEndpoint);
        }
        self.notifier.validate_endpoint(&cmd.endpoint_url).await?;

        let connection = match self
            .connections
            .find_connection_by_service(cmd.team_id, HTTP_SERVICE)
            .await?
        {
            Some(connection) => connection,
            None => {
                let connection =
                    ServiceConnection::new(cmd.team_id, HTTP_SERVICE, cmd.requester_id)?;
                self.connections.insert_connection(&connection).await?;
                connection
            }
        };
        self.credentials
            .store_credential(
                connection.id,
                CredentialKind::EndpointUrl,
                &cmd.endpoint_url,
            )
            .await?;
        self.connections
            .reset_connection_health(connection.id)
            .await?;
        self.connection_view(cmd.team_id, connection.id).await
    }

    pub async fn configure_email(
        &self,
        cmd: ConfigureEmailConnectionCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        validate_optional_secret(&cmd.smtp_username)?;
        validate_optional_secret(&cmd.smtp_password)?;

        if let Some(port) = &cmd.smtp_port {
            port.trim()
                .parse::<u16>()
                .map_err(|_| DomainError::InvalidReactionEndpoint)?;
        }
        if let Some(host) = &cmd.smtp_host {
            if host.trim().is_empty() {
                return Err(DomainError::InvalidReactionEndpoint);
            }
        }
        // Catch an obviously malformed sender before it reaches the SMTP relay.
        if let Some(from) = &cmd.from_address {
            Email::new(from.trim()).map_err(|_| DomainError::InvalidEmailSender)?;
        }

        let existing = self
            .connections
            .find_connection_by_service(cmd.team_id, EMAIL_SERVICE)
            .await?;
        // A partially configured connection could never authenticate, so refuse
        // to create one rather than surface a confusing SMTP error later.
        if existing.is_none()
            && (cmd.smtp_host.is_none()
                || cmd.smtp_port.is_none()
                || cmd.smtp_username.is_none()
                || cmd.smtp_password.is_none()
                || cmd.from_address.is_none())
        {
            return Err(DomainError::InvalidServiceSecret);
        }

        let connection = match existing {
            Some(connection) => connection,
            None => {
                let connection =
                    ServiceConnection::new(cmd.team_id, EMAIL_SERVICE, cmd.requester_id)?;
                self.connections.insert_connection(&connection).await?;
                connection
            }
        };

        for (kind, value) in [
            (CredentialKind::SmtpHost, cmd.smtp_host),
            (CredentialKind::SmtpPort, cmd.smtp_port),
            (CredentialKind::SmtpUsername, cmd.smtp_username),
            (CredentialKind::SmtpPassword, cmd.smtp_password),
            (CredentialKind::FromAddress, cmd.from_address),
        ] {
            if let Some(value) = value {
                self.credentials
                    .store_credential(connection.id, kind, value.trim())
                    .await?;
            }
        }
        self.connections
            .reset_connection_health(connection.id)
            .await?;
        self.connection_view(cmd.team_id, connection.id).await
    }

    /// Exercise a testable connection without emitting a business notification,
    /// and record the outcome on the connection's health the same way a real
    /// REAction would. Only services advertised as `testable` in the catalogue
    /// reach a probe; anything else is refused rather than silently accepted.
    pub async fn test(&self, cmd: TestConnectionCommand) -> Result<(), DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let connection = self
            .connections
            .find_connection_for_team(cmd.team_id, cmd.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;

        let probe = service(&connection.service)
            .and_then(|definition| definition.connection)
            .and_then(|connection| connection.probe)
            .ok_or(DomainError::InvalidServiceConnection)?;
        let outcome = match probe {
            ConnectionProbe::Http => self.probe_http(connection.id).await,
            ConnectionProbe::Email => self.probe_smtp(connection.id).await,
        };

        match outcome {
            Ok(()) => {
                self.connections
                    .record_reaction_result(connection.id, None)
                    .await
            }
            Err(error) => {
                let _ = self
                    .connections
                    .record_reaction_result(connection.id, Some(error.code()))
                    .await;
                Err(error)
            }
        }
    }

    async fn probe_http(&self, connection_id: Uuid) -> Result<(), DomainError> {
        let endpoint = self
            .credentials
            .reveal_credential(connection_id, CredentialKind::EndpointUrl)
            .await?
            .ok_or(DomainError::InvalidReactionEndpoint)?;
        self.notifier
            .notify(&endpoint, CONNECTION_TEST_MESSAGE)
            .await
    }

    /// Open an authenticated SMTP session and issue NOOP. No message is sent, so
    /// a Manager can validate credentials without mailing anyone.
    async fn probe_smtp(&self, connection_id: Uuid) -> Result<(), DomainError> {
        let config = smtp_config(self.credentials.as_ref(), connection_id).await?;
        self.email_sender.validate_smtp(&config).await
    }

    pub async fn list(
        &self,
        cmd: ListTeamConnectionsCommand,
    ) -> Result<Vec<TeamConnectionView>, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let connections = self
            .connections
            .list_connections_for_team(cmd.team_id)
            .await?;
        let mut views = Vec::with_capacity(connections.len());
        for connection in connections {
            let configured_credentials = self
                .credentials
                .configured_credential_kinds(connection.id)
                .await?;
            views.push(TeamConnectionView {
                connection,
                configured_credentials,
            });
        }
        Ok(views)
    }

    pub async fn delete(&self, cmd: DeleteTeamConnectionCommand) -> Result<(), DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let connection = self
            .connections
            .find_connection_for_team(cmd.team_id, cmd.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if service(&connection.service).is_some_and(|definition| definition.internal) {
            return Err(DomainError::InvalidServiceConnection);
        }
        if !self
            .connections
            .delete_connection(cmd.team_id, cmd.connection_id)
            .await?
        {
            return Err(DomainError::ServiceConnectionNotFound);
        }
        Ok(())
    }

    async fn connection_view(
        &self,
        team_id: Uuid,
        connection_id: Uuid,
    ) -> Result<TeamConnectionView, DomainError> {
        let connection = self
            .connections
            .find_connection_for_team(team_id, connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        let configured_credentials = self
            .credentials
            .configured_credential_kinds(connection_id)
            .await?;
        Ok(TeamConnectionView {
            connection,
            configured_credentials,
        })
    }
}

fn validate_optional_secret(value: &Option<String>) -> Result<(), DomainError> {
    if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
        return Err(DomainError::InvalidServiceSecret);
    }
    Ok(())
}
