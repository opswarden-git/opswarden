use std::sync::Arc;

use crate::domain::error::DomainError;
use crate::ports::WebhookJobRepo;

use super::{IngestTeamWebhookUseCase, TeamWebhookDependencies};

pub struct WebhookWorker {
    jobs: Arc<dyn WebhookJobRepo>,
    use_case: IngestTeamWebhookUseCase,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WebhookTickResult {
    pub claimed: usize,
    pub completed: usize,
    pub retried: usize,
}

impl WebhookWorker {
    pub fn new(dependencies: TeamWebhookDependencies, jobs: Arc<dyn WebhookJobRepo>) -> Self {
        Self {
            jobs,
            use_case: IngestTeamWebhookUseCase::new(dependencies),
        }
    }

    pub async fn tick(&self) -> Result<WebhookTickResult, DomainError> {
        let claims = self.jobs.claim(16).await?;
        let mut result = WebhookTickResult {
            claimed: claims.len(),
            ..WebhookTickResult::default()
        };
        for claim in claims {
            match self.use_case.process_job(claim.job.clone()).await {
                Ok(_) => {
                    if !self.jobs.complete(&claim).await? {
                        return Err(DomainError::InvalidAutomationTransition);
                    }
                    result.completed += 1;
                }
                Err(error) => {
                    if !self.jobs.retry(&claim, error.code()).await? {
                        return Err(DomainError::InvalidAutomationTransition);
                    }
                    result.retried += 1;
                }
            }
        }
        Ok(result)
    }
}
