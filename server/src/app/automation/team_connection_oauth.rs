use std::sync::Arc;

use uuid::Uuid;

use super::team_access::require_manager;
use super::team_connection::TeamConnectionView;
use crate::domain::automation_catalog::GITHUB_SERVICE;
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::ports::{
    ConnectionCredentialVault, ServiceConnectionRepo, ServiceOAuthClient, TeamRepo,
};

pub struct StartGithubOAuthCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub state: String,
    pub code_challenge: String,
}

pub struct CompleteGithubOAuthCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub code: String,
    pub code_verifier: String,
}

pub struct RefreshGithubOAuthCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub connection_id: Uuid,
}

/// GitHub's authorization-code flow for Team-owned service credentials.
/// Provider tokens never leave this use case except through the encrypted vault.
pub struct TeamConnectionOAuthUseCase {
    teams: Arc<dyn TeamRepo>,
    connections: Arc<dyn ServiceConnectionRepo>,
    credentials: Arc<dyn ConnectionCredentialVault>,
    oauth: Arc<dyn ServiceOAuthClient>,
}

impl TeamConnectionOAuthUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        connections: Arc<dyn ServiceConnectionRepo>,
        credentials: Arc<dyn ConnectionCredentialVault>,
        oauth: Arc<dyn ServiceOAuthClient>,
    ) -> Self {
        Self {
            teams,
            connections,
            credentials,
            oauth,
        }
    }

    pub async fn start(&self, cmd: StartGithubOAuthCommand) -> Result<String, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        if !self.oauth.is_configured() {
            return Err(DomainError::OAuthNotConfigured);
        }
        self.oauth
            .authorization_url(&cmd.state, &cmd.code_challenge)
    }

    pub async fn complete(
        &self,
        cmd: CompleteGithubOAuthCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let tokens = self
            .oauth
            .exchange_code(&cmd.code, &cmd.code_verifier)
            .await?;
        let connection = self
            .github_connection(cmd.team_id, cmd.requester_id)
            .await?;

        self.credentials
            .store_credential(
                connection.id,
                CredentialKind::OAuthAccessToken,
                &tokens.access_token,
            )
            .await?;
        if let Some(refresh_token) = tokens.refresh_token {
            self.credentials
                .store_credential(
                    connection.id,
                    CredentialKind::OAuthRefreshToken,
                    &refresh_token,
                )
                .await?;
        } else {
            // Reauthorization must not leave an old refresh token paired with a
            // newly issued access token that cannot be refreshed with it.
            self.credentials
                .delete_credential(connection.id, CredentialKind::OAuthRefreshToken)
                .await?;
        }
        self.connections
            .record_reaction_result(connection.id, None)
            .await?;
        self.connection_view(cmd.team_id, connection.id).await
    }

    pub async fn refresh(
        &self,
        cmd: RefreshGithubOAuthCommand,
    ) -> Result<TeamConnectionView, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let connection = self
            .connections
            .find_connection_for_team(cmd.team_id, cmd.connection_id)
            .await?
            .filter(|connection| connection.service == GITHUB_SERVICE)
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        let current_refresh = self
            .credentials
            .reveal_credential(connection.id, CredentialKind::OAuthRefreshToken)
            .await?
            .ok_or(DomainError::OAuthFailed)?;
        let tokens = self.oauth.refresh_access_token(&current_refresh).await?;

        self.credentials
            .store_credential(
                connection.id,
                CredentialKind::OAuthAccessToken,
                &tokens.access_token,
            )
            .await?;
        if let Some(rotated_refresh) = tokens.refresh_token {
            self.credentials
                .store_credential(
                    connection.id,
                    CredentialKind::OAuthRefreshToken,
                    &rotated_refresh,
                )
                .await?;
        }
        self.connections
            .record_reaction_result(connection.id, None)
            .await?;
        self.connection_view(cmd.team_id, connection.id).await
    }

    async fn github_connection(
        &self,
        team_id: Uuid,
        requester_id: Uuid,
    ) -> Result<ServiceConnection, DomainError> {
        match self
            .connections
            .find_connection_by_service(team_id, GITHUB_SERVICE)
            .await?
        {
            Some(connection) => Ok(connection),
            None => {
                let connection = ServiceConnection::new(team_id, GITHUB_SERVICE, requester_id)?;
                self.connections.insert_connection(&connection).await?;
                Ok(connection)
            }
        }
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
