use std::sync::Arc;

use uuid::Uuid;

use super::team_access::require_manager;
use crate::domain::automation_config::AutomationRun;
use crate::domain::error::DomainError;
use crate::ports::{AutomationRunRepo, TeamRepo};

pub struct ListTeamRunsCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub limit: u32,
}

pub struct TeamRunUseCase {
    teams: Arc<dyn TeamRepo>,
    runs: Arc<dyn AutomationRunRepo>,
}

impl TeamRunUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, runs: Arc<dyn AutomationRunRepo>) -> Self {
        Self { teams, runs }
    }

    pub async fn list(&self, cmd: ListTeamRunsCommand) -> Result<Vec<AutomationRun>, DomainError> {
        require_manager(&self.teams, cmd.team_id, cmd.requester_id).await?;
        self.runs.list_runs_for_team(cmd.team_id, cmd.limit).await
    }
}
