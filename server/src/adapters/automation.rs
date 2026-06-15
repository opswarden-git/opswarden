// --- server/src/adapters/automation.rs ---
//
// Rule source for the hook engine. Phase 2 ships a single in-memory rule; a
// DB-backed or API-managed rule set later just implements `RuleRepo` without
// touching the engine or the use-case.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::automation::{Reaction, Rule};
use crate::domain::error::DomainError;
use crate::domain::incident::Severity;
use crate::ports::RuleRepo;

/// Rule names, surfaced on `rule_triggered` / `rule_failed` events.
pub const GITHUB_CI_RULE: &str = "github-ci-failed-to-incident";
pub const GITHUB_NOTIFY_RULE: &str = "github-ci-failed-to-notify";

pub struct StaticRuleRepo {
    rules: Vec<Rule>,
}

impl StaticRuleRepo {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// The Phase-2 end-to-end rule: a failed GitHub CI run opens a `high`
    /// incident in `team_id`.
    pub fn github_ci_to_incident(team_id: Uuid) -> Self {
        Self::new(vec![incident_rule(team_id)])
    }

    /// On a failed GitHub CI run, open an incident AND (when a notify URL is set)
    /// POST a notification. Two rules on the same trigger: `evaluate()` fires both
    /// — the "CI breaks -> incident + Slack ping" demo, with no engine change.
    pub fn github_ci_rules(team_id: Uuid, notify_url: Option<String>) -> Self {
        let mut rules = vec![incident_rule(team_id)];
        if let Some(url) = notify_url {
            rules.push(Rule {
                name: GITHUB_NOTIFY_RULE.to_string(),
                on_service: "github".to_string(),
                on_kind: "ci_failed".to_string(),
                reaction: Reaction::Notify {
                    team_id,
                    url,
                    message: "OpsWarden: GitHub CI failed — a high-severity incident was opened."
                        .to_string(),
                },
            });
        }
        Self::new(rules)
    }
}

fn incident_rule(team_id: Uuid) -> Rule {
    Rule {
        name: GITHUB_CI_RULE.to_string(),
        on_service: "github".to_string(),
        on_kind: "ci_failed".to_string(),
        reaction: Reaction::CreateIncident {
            team_id,
            severity: Severity::High,
            title: "CI failed on GitHub".to_string(),
        },
    }
}

#[async_trait]
impl RuleRepo for StaticRuleRepo {
    async fn list_rules(&self) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.clone())
    }
}
