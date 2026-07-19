use std::sync::Arc;

use uuid::Uuid;

use super::team_access::require_manager;
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::ports::{ConnectionCredentialVault, Notifier, ServiceConnectionRepo, TeamRepo};

pub const GITHUB_SERVICE: &str = "github";
pub const HTTP_SERVICE: &str = "http";
const CONNECTION_TEST_MESSAGE: &str = "OpsWarden connection test";

pub struct ConfigureGithubConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub webhook_signing_secret: Option<String>,
    pub personal_token: Option<String>,
}

pub struct ConfigureHttpConnectionCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub endpoint_url: String,
}

pub struct TestHttpConnectionCommand {
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
}

impl TeamConnectionUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        connections: Arc<dyn ServiceConnectionRepo>,
        credentials: Arc<dyn ConnectionCredentialVault>,
        notifier: Arc<dyn Notifier>,
    ) -> Self {
        Self {
            teams,
            connections,
            credentials,
            notifier,
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

    pub async fn test_http(&self, cmd: TestHttpConnectionCommand) -> Result<(), DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let connection = self
            .connections
            .find_connection_for_team(cmd.team_id, cmd.connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if connection.service != HTTP_SERVICE {
            return Err(DomainError::InvalidServiceConnection);
        }
        let endpoint = self
            .credentials
            .reveal_credential(connection.id, CredentialKind::EndpointUrl)
            .await?
            .ok_or(DomainError::InvalidReactionEndpoint)?;

        match self
            .notifier
            .notify(&endpoint, CONNECTION_TEST_MESSAGE)
            .await
        {
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
