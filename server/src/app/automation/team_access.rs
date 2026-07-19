use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::TeamRepo;

pub(super) async fn require_manager(
    teams: &Arc<dyn TeamRepo>,
    team_id: Uuid,
    requester_id: Uuid,
) -> Result<(), DomainError> {
    let role = teams
        .find_member_role(team_id, requester_id)
        .await?
        .ok_or(DomainError::Forbidden)?;
    if role != Role::Manager {
        return Err(DomainError::NotManager);
    }
    Ok(())
}
