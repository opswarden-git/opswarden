use std::sync::Arc;

use uuid::Uuid;

use super::team_access::require_manager;
use super::team_connection::TeamConnectionView;
use crate::domain::automation_catalog::GITHUB_SERVICE;
use crate::domain::automation_config::{CredentialKind, ServiceConnection};
use crate::domain::error::DomainError;
use crate::ports::{
    ConnectionCredentialVault, ConnectionHealthMutation, CredentialMutation, ServiceConnectionRepo,
    ServiceOAuthClient, TeamRepo,
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
        let candidate = match self
            .connections
            .find_connection_by_service(cmd.team_id, GITHUB_SERVICE)
            .await?
        {
            Some(connection) => connection,
            None => ServiceConnection::new(cmd.team_id, GITHUB_SERVICE, cmd.requester_id)?,
        };
        let mutations = [
            CredentialMutation {
                kind: CredentialKind::OAuthAccessToken,
                secret: Some(tokens.access_token),
            },
            CredentialMutation {
                kind: CredentialKind::OAuthRefreshToken,
                secret: tokens.refresh_token,
            },
        ];
        let connection = self
            .credentials
            .configure_connection(&candidate, &mutations, ConnectionHealthMutation::Verified)
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

        let mut mutations = vec![CredentialMutation {
            kind: CredentialKind::OAuthAccessToken,
            secret: Some(tokens.access_token),
        }];
        if let Some(rotated_refresh) = tokens.refresh_token {
            mutations.push(CredentialMutation {
                kind: CredentialKind::OAuthRefreshToken,
                secret: Some(rotated_refresh),
            });
        }
        let connection = self
            .credentials
            .configure_connection(&connection, &mutations, ConnectionHealthMutation::Verified)
            .await?;
        self.connection_view(cmd.team_id, connection.id).await
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
