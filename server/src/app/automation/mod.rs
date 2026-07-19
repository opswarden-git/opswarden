// --- server/src/app/automation/mod.rs ---

pub mod ingest_team_webhook;
pub mod reaction_executor;
mod team_access;
pub mod team_connection;
pub mod team_rule;
pub mod team_run;

pub use ingest_team_webhook::{
    IngestTeamWebhookCommand, IngestTeamWebhookResult, IngestTeamWebhookUseCase,
    TeamWebhookDependencies,
};
pub use reaction_executor::AutomationReactionExecutor;
pub use team_connection::{
    ConfigureGithubConnectionCommand, ConfigureHttpConnectionCommand, DeleteTeamConnectionCommand,
    ListTeamConnectionsCommand, TeamConnectionUseCase, TeamConnectionView,
    TestHttpConnectionCommand,
};
pub use team_rule::{
    CreateTeamRuleCommand, DeleteTeamRuleCommand, ListTeamRulesCommand, TeamRuleUseCase,
    UpdateTeamRuleCommand,
};
pub use team_run::{ListTeamRunsCommand, TeamRunUseCase};
