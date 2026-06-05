// --- server/src/app/team/transfer_manager.rs ---
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::plan_manager_transfer;
use crate::ports::TeamRepo;

pub struct TransferManagerCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub new_manager_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransferManagerResult {
    pub team_id: Uuid,
    pub new_manager_id: Uuid,
}

pub struct TransferManagerUseCase {
    teams: Arc<dyn TeamRepo>,
}

impl TransferManagerUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>) -> Self {
        Self { teams }
    }

    /// Hand over management. RBAC is enforced from the requester's actual role
    /// (403 if not Manager); the target must already be a member (404); the
    /// pure invariant then yields the atomic demote+promote applied by the repo.
    pub async fn transfer_manager(
        &self,
        cmd: TransferManagerCommand,
    ) -> Result<TransferManagerResult, DomainError> {
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::NotManager)?;

        if self
            .teams
            .find_member_role(cmd.team_id, cmd.new_manager_id)
            .await?
            .is_none()
        {
            return Err(DomainError::MemberNotFound);
        }

        // Single-Manager invariant lives in the domain, not here.
        plan_manager_transfer(requester_role, cmd.requester_id, cmd.new_manager_id)?;

        self.teams
            .transfer_manager(cmd.team_id, cmd.requester_id, cmd.new_manager_id)
            .await?;
        Ok(TransferManagerResult {
            team_id: cmd.team_id,
            new_manager_id: cmd.new_manager_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team::tests::MockTeamRepo;
    use crate::domain::team::Role;

    #[tokio::test]
    async fn manager_transfers_to_another_member() {
        let team_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(responder, Role::Responder),
        );
        let use_case = TransferManagerUseCase::new(repo.clone());

        let result = use_case
            .transfer_manager(TransferManagerCommand {
                team_id,
                requester_id: manager,
                new_manager_id: responder,
            })
            .await
            .unwrap();

        assert_eq!(result.new_manager_id, responder);
        let transfers = repo.transfers.lock().unwrap();
        assert_eq!(transfers.as_slice(), &[(team_id, manager, responder)]);
    }

    #[tokio::test]
    async fn non_manager_is_refused_with_not_manager() {
        let observer = Uuid::new_v4();
        let other = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(observer, Role::Observer)
                .with_member(other, Role::Responder),
        );
        let use_case = TransferManagerUseCase::new(repo.clone());

        let result = use_case
            .transfer_manager(TransferManagerCommand {
                team_id: Uuid::new_v4(),
                requester_id: observer,
                new_manager_id: other,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
        assert!(repo.transfers.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn transfer_to_non_member_is_rejected() {
        let manager = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let use_case = TransferManagerUseCase::new(repo.clone());

        let result = use_case
            .transfer_manager(TransferManagerCommand {
                team_id: Uuid::new_v4(),
                requester_id: manager,
                new_manager_id: stranger,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::MemberNotFound);
        assert!(repo.transfers.lock().unwrap().is_empty());
    }
}
