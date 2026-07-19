use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use super::team_access::require_manager;
use crate::domain::automation_catalog::{reaction, supports_action};
use crate::domain::automation_config::{AutomationRule, AutomationRuleDefinition};
use crate::domain::error::DomainError;
use crate::ports::{AutomationRuleRepo, ServiceConnectionRepo, TeamRepo};

pub struct ListTeamRulesCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct CreateTeamRuleCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub definition: AutomationRuleDefinition,
}

pub struct UpdateTeamRuleCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub rule_id: Uuid,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger_connection_id: Option<Uuid>,
    pub trigger_kind: Option<String>,
    pub trigger_config: Option<Value>,
    pub reaction_kind: Option<String>,
    pub reaction_connection_id: Option<Option<Uuid>>,
    pub reaction_config: Option<Value>,
}

pub struct DeleteTeamRuleCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub rule_id: Uuid,
}

pub struct TeamRuleUseCase {
    teams: Arc<dyn TeamRepo>,
    connections: Arc<dyn ServiceConnectionRepo>,
    rules: Arc<dyn AutomationRuleRepo>,
}

impl TeamRuleUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        connections: Arc<dyn ServiceConnectionRepo>,
        rules: Arc<dyn AutomationRuleRepo>,
    ) -> Self {
        Self {
            teams,
            connections,
            rules,
        }
    }

    pub async fn list(
        &self,
        cmd: ListTeamRulesCommand,
    ) -> Result<Vec<AutomationRule>, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        self.rules.list_rules_for_team(cmd.team_id).await
    }

    pub async fn create(&self, cmd: CreateTeamRuleCommand) -> Result<AutomationRule, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        self.validate_definition(cmd.team_id, &cmd.definition)
            .await?;
        let rule = AutomationRule::new(
            cmd.team_id,
            cmd.definition.name,
            cmd.definition.trigger_connection_id,
            cmd.definition.trigger_kind,
            cmd.definition.trigger_config,
            cmd.definition.reaction_kind,
            cmd.definition.reaction_connection_id,
            cmd.definition.reaction_config,
            cmd.requester_id,
        )?;
        self.rules.insert_rule(&rule).await?;
        Ok(rule)
    }

    pub async fn update(&self, cmd: UpdateTeamRuleCommand) -> Result<AutomationRule, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        let mut rule = self
            .rules
            .find_rule_for_team(cmd.team_id, cmd.rule_id)
            .await?
            .ok_or(DomainError::AutomationRuleNotFound)?;
        let current = rule.definition();
        let definition = AutomationRuleDefinition {
            name: cmd.name.unwrap_or(current.name),
            trigger_connection_id: cmd
                .trigger_connection_id
                .unwrap_or(current.trigger_connection_id),
            trigger_kind: cmd.trigger_kind.unwrap_or(current.trigger_kind),
            trigger_config: cmd.trigger_config.unwrap_or(current.trigger_config),
            reaction_kind: cmd.reaction_kind.unwrap_or(current.reaction_kind),
            reaction_connection_id: cmd
                .reaction_connection_id
                .unwrap_or(current.reaction_connection_id),
            reaction_config: cmd.reaction_config.unwrap_or(current.reaction_config),
        };
        self.validate_definition(cmd.team_id, &definition).await?;
        rule.replace_definition(definition)?;
        if let Some(enabled) = cmd.enabled {
            rule.set_enabled(enabled);
        }
        if !self.rules.update_rule(&rule).await? {
            return Err(DomainError::AutomationRuleNotFound);
        }
        Ok(rule)
    }

    pub async fn delete(&self, cmd: DeleteTeamRuleCommand) -> Result<(), DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        if !self.rules.delete_rule(cmd.team_id, cmd.rule_id).await? {
            return Err(DomainError::AutomationRuleNotFound);
        }
        Ok(())
    }

    async fn validate_definition(
        &self,
        team_id: Uuid,
        definition: &AutomationRuleDefinition,
    ) -> Result<(), DomainError> {
        let trigger = self
            .connections
            .find_connection_for_team(team_id, definition.trigger_connection_id)
            .await?
            .ok_or(DomainError::ServiceConnectionNotFound)?;
        if !supports_action(&trigger.service, &definition.trigger_kind) {
            return Err(DomainError::InvalidAutomationRule);
        }

        let reaction =
            reaction(&definition.reaction_kind).ok_or(DomainError::InvalidAutomationRule)?;
        if definition.reaction_kind == "http_notify"
            && definition.reaction_config != serde_json::json!({})
        {
            return Err(DomainError::InvalidAutomationRule);
        }
        match (
            reaction.connection_service,
            definition.reaction_connection_id,
        ) {
            (None, None) => Ok(()),
            (Some(expected_service), Some(connection_id)) => {
                let connection = self
                    .connections
                    .find_connection_for_team(team_id, connection_id)
                    .await?
                    .ok_or(DomainError::ServiceConnectionNotFound)?;
                if connection.service != expected_service {
                    return Err(DomainError::InvalidAutomationRule);
                }
                Ok(())
            }
            _ => Err(DomainError::InvalidAutomationRule),
        }
    }
}
