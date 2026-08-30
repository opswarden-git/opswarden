// --- server/src/app/automation/mod.rs ---

pub mod dispatch_internal_event;
pub mod ingest_team_webhook;
pub mod reaction_executor;
mod team_access;
pub mod team_connection;
pub mod team_connection_oauth;
pub mod team_rule;
pub mod team_run;
pub mod timer_worker;
pub mod webhook_worker;

pub use dispatch_internal_event::{
    release_created_event, DispatchInternalAutomationCommand, DispatchInternalAutomationResult,
    DispatchInternalAutomationUseCase, InternalAutomationDependencies,
};
pub use ingest_team_webhook::{
    DurableTeamWebhookIngress, IngestTeamWebhookCommand, IngestTeamWebhookResult,
    IngestTeamWebhookUseCase, TeamWebhookDependencies, TeamWebhookIngress,
};
pub use reaction_executor::AutomationReactionExecutor;
pub use team_connection::{
    ConfigureEmailConnectionCommand, ConfigureGithubConnectionCommand,
    ConfigureHttpConnectionCommand, ConfigureTokenWebhookConnectionCommand,
    DeleteTeamConnectionCommand, ListTeamConnectionsCommand, TeamConnectionUseCase,
    TeamConnectionView, TestConnectionCommand,
};
pub use team_connection_oauth::{
    CompleteGithubOAuthCommand, RefreshGithubOAuthCommand, StartGithubOAuthCommand,
    TeamConnectionOAuthUseCase,
};
pub use team_rule::{
    CreateTeamRuleCommand, DeleteTeamRuleCommand, ListTeamRulesCommand, TeamRuleUseCase,
    UpdateTeamRuleCommand,
};
pub use team_run::{ListTeamRunsCommand, TeamRunUseCase};
pub use timer_worker::{
    TimerReconcileResult, TimerTickResult, TimerWorker, TimerWorkerDependencies,
};
pub use webhook_worker::{WebhookTickResult, WebhookWorker};
