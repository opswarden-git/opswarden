use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::app::automation::{
    ConfigureGithubConnectionCommand, ConfigureHttpConnectionCommand, CreateTeamRuleCommand,
    DeleteTeamConnectionCommand, DeleteTeamRuleCommand, ListTeamConnectionsCommand,
    ListTeamRulesCommand, ListTeamRunsCommand, TeamConnectionUseCase, TeamConnectionView,
    TeamRuleUseCase, TeamRunUseCase, TestHttpConnectionCommand, UpdateTeamRuleCommand,
};
use crate::domain::automation_config::{AutomationRule, AutomationRuleDefinition, AutomationRun};
use crate::domain::error::DomainError;
use crate::handlers::middleware::AuthenticatedSession;
use crate::AppState;

#[derive(Deserialize)]
pub struct ConfigureGithubPayload {
    pub webhook_signing_secret: Option<String>,
    pub personal_token: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfigureHttpPayload {
    pub endpoint_url: String,
}

#[derive(Serialize)]
pub struct TeamConnectionResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub service: String,
    pub secret_configured: bool,
    pub token_configured: bool,
    pub endpoint_configured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub webhook_path: Option<String>,
}

impl From<TeamConnectionView> for TeamConnectionResponse {
    fn from(view: TeamConnectionView) -> Self {
        use crate::domain::automation_config::CredentialKind;

        let webhook_path = (view.connection.service == "github")
            .then(|| format!("/webhooks/github/{}", view.connection.id));
        Self {
            id: view.connection.id,
            team_id: view.connection.team_id,
            service: view.connection.service,
            secret_configured: view
                .configured_credentials
                .contains(&CredentialKind::WebhookSigningSecret),
            token_configured: view
                .configured_credentials
                .contains(&CredentialKind::PersonalToken),
            endpoint_configured: view
                .configured_credentials
                .contains(&CredentialKind::EndpointUrl),
            created_at: view.connection.created_at,
            updated_at: view.connection.updated_at,
            verified_at: view.connection.verified_at,
            last_delivery_at: view.connection.last_delivery_at,
            last_error_code: view.connection.last_error_code,
            webhook_path,
        }
    }
}

pub async fn list_connections(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<TeamConnectionResponse>>, DomainError> {
    let views = TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
    )
    .list(ListTeamConnectionsCommand {
        team_id,
        requester_id: session.user_id,
    })
    .await?;
    Ok(Json(views.into_iter().map(Into::into).collect()))
}

pub async fn configure_github(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<ConfigureGithubPayload>,
) -> Result<Json<TeamConnectionResponse>, DomainError> {
    let view = TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
    )
    .configure_github(ConfigureGithubConnectionCommand {
        team_id,
        requester_id: session.user_id,
        webhook_signing_secret: payload.webhook_signing_secret,
        personal_token: payload.personal_token,
    })
    .await?;
    Ok(Json(view.into()))
}

pub async fn configure_http(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<ConfigureHttpPayload>,
) -> Result<Json<TeamConnectionResponse>, DomainError> {
    let view = TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
    )
    .configure_http(ConfigureHttpConnectionCommand {
        team_id,
        requester_id: session.user_id,
        endpoint_url: payload.endpoint_url,
    })
    .await?;
    Ok(Json(view.into()))
}

pub async fn test_http_connection(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
    )
    .test_http(TestHttpConnectionCommand {
        team_id,
        requester_id: session.user_id,
        connection_id,
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_connection(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    TeamConnectionUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.connection_credentials.clone(),
        state.notifier.clone(),
    )
    .delete(DeleteTeamConnectionCommand {
        team_id,
        requester_id: session.user_id,
        connection_id,
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct CreateRulePayload {
    pub name: String,
    pub trigger_connection_id: Uuid,
    pub trigger_kind: String,
    #[serde(default = "empty_object")]
    pub trigger_config: Value,
    pub reaction_kind: String,
    pub reaction_connection_id: Option<Uuid>,
    #[serde(default = "empty_object")]
    pub reaction_config: Value,
}

#[derive(Deserialize)]
pub struct UpdateRulePayload {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger_connection_id: Option<Uuid>,
    pub trigger_kind: Option<String>,
    pub trigger_config: Option<Value>,
    pub reaction_kind: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_uuid")]
    pub reaction_connection_id: Option<Option<Uuid>>,
    pub reaction_config: Option<Value>,
}

fn deserialize_nullable_uuid<'de, D>(deserializer: D) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Uuid>::deserialize(deserializer).map(Some)
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

#[derive(Serialize)]
pub struct AutomationRuleResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub trigger_connection_id: Uuid,
    pub trigger_kind: String,
    pub trigger_config: Value,
    pub reaction_kind: String,
    pub reaction_connection_id: Option<Uuid>,
    pub reaction_config: Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AutomationRule> for AutomationRuleResponse {
    fn from(rule: AutomationRule) -> Self {
        Self {
            id: rule.id,
            team_id: rule.team_id,
            name: rule.name,
            enabled: rule.enabled,
            trigger_connection_id: rule.trigger_connection_id,
            trigger_kind: rule.trigger_kind,
            trigger_config: rule.trigger_config,
            reaction_kind: rule.reaction_kind,
            reaction_connection_id: rule.reaction_connection_id,
            reaction_config: rule.reaction_config,
            created_by: rule.created_by,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }
    }
}

pub async fn list_rules(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<AutomationRuleResponse>>, DomainError> {
    let rules = TeamRuleUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.automation_rules.clone(),
    )
    .list(ListTeamRulesCommand {
        team_id,
        requester_id: session.user_id,
    })
    .await?;
    Ok(Json(rules.into_iter().map(Into::into).collect()))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<CreateRulePayload>,
) -> Result<(StatusCode, Json<AutomationRuleResponse>), DomainError> {
    let rule = TeamRuleUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.automation_rules.clone(),
    )
    .create(CreateTeamRuleCommand {
        team_id,
        requester_id: session.user_id,
        definition: AutomationRuleDefinition {
            name: payload.name,
            trigger_connection_id: payload.trigger_connection_id,
            trigger_kind: payload.trigger_kind,
            trigger_config: payload.trigger_config,
            reaction_kind: payload.reaction_kind,
            reaction_connection_id: payload.reaction_connection_id,
            reaction_config: payload.reaction_config,
        },
    })
    .await?;
    Ok((StatusCode::CREATED, Json(rule.into())))
}

pub async fn update_rule(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, rule_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRulePayload>,
) -> Result<Json<AutomationRuleResponse>, DomainError> {
    let rule = TeamRuleUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.automation_rules.clone(),
    )
    .update(UpdateTeamRuleCommand {
        team_id,
        requester_id: session.user_id,
        rule_id,
        name: payload.name,
        enabled: payload.enabled,
        trigger_connection_id: payload.trigger_connection_id,
        trigger_kind: payload.trigger_kind,
        trigger_config: payload.trigger_config,
        reaction_kind: payload.reaction_kind,
        reaction_connection_id: payload.reaction_connection_id,
        reaction_config: payload.reaction_config,
    })
    .await?;
    Ok(Json(rule.into()))
}

pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path((team_id, rule_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, DomainError> {
    TeamRuleUseCase::new(
        state.teams.clone(),
        state.service_connections.clone(),
        state.automation_rules.clone(),
    )
    .delete(DeleteTeamRuleCommand {
        team_id,
        requester_id: session.user_id,
        rule_id,
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ListRunsQuery {
    #[serde(default = "default_run_limit")]
    pub limit: u32,
}

fn default_run_limit() -> u32 {
    50
}

#[derive(Serialize)]
pub struct AutomationRunResponse {
    pub id: Uuid,
    pub delivery_id: Uuid,
    pub rule_id: Option<Uuid>,
    pub status: String,
    pub incident_id: Option<Uuid>,
    pub error_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<AutomationRun> for AutomationRunResponse {
    fn from(run: AutomationRun) -> Self {
        Self {
            id: run.id,
            delivery_id: run.delivery_id,
            rule_id: run.rule_id,
            status: run.status.to_string(),
            incident_id: run.incident_id,
            error_code: run.error_code,
            started_at: run.started_at,
            finished_at: run.finished_at,
        }
    }
}

pub async fn list_runs(
    State(state): State<AppState>,
    Extension(session): Extension<AuthenticatedSession>,
    Path(team_id): Path<Uuid>,
    Query(query): Query<ListRunsQuery>,
) -> Result<Json<Vec<AutomationRunResponse>>, DomainError> {
    let runs = TeamRunUseCase::new(state.teams.clone(), state.automation_runs.clone())
        .list(ListTeamRunsCommand {
            team_id,
            requester_id: session.user_id,
            limit: query.limit,
        })
        .await?;
    Ok(Json(runs.into_iter().map(Into::into).collect()))
}
