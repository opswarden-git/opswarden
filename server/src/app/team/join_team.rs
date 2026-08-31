// --- server/src/app/team/join_team.rs ---
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::Role;
use crate::ports::{Clock, TeamRepo};

pub struct JoinTeamCommand {
    pub invitation_code: String,
    pub user_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct JoinTeamResult {
    pub team_id: Uuid,
    pub role: Role,
}

pub struct JoinTeamUseCase {
    teams: Arc<dyn TeamRepo>,
    clock: Arc<dyn Clock>,
}

/// Role granted to anyone joining via invitation code: least privilege first.
/// Promotion to Responder/Manager is a deliberate, separate action.
const JOIN_ROLE: Role = Role::Observer;

impl JoinTeamUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { teams, clock }
    }

    /// Join a team from its invitation code as an `Observer`. Unknown codes are
    /// rejected (404) and double-joining is refused (409).
    pub async fn join_team(&self, cmd: JoinTeamCommand) -> Result<JoinTeamResult, DomainError> {
        let normalized_code = cmd.invitation_code.trim().to_uppercase();
        let team = self
            .teams
            .find_by_invitation_code(&normalized_code)
            .await?
            .ok_or(DomainError::TeamNotFound)?;

        // An active ban blocks (re)joining. An expired temporary ban does not —
        // the row may linger, but `is_active` gates on its expiry.
        if let Some(ban) = self.teams.find_ban(team.id, cmd.user_id).await? {
            if ban.is_active(self.clock.now()) {
                return Err(DomainError::UserBanned);
            }
        }

        if self
            .teams
            .find_member_role(team.id, cmd.user_id)
            .await?
            .is_some()
        {
            return Err(DomainError::AlreadyMember);
        }

        self.teams
            .add_member(team.id, cmd.user_id, JOIN_ROLE)
            .await?;
        Ok(JoinTeamResult {
            team_id: team.id,
            role: JOIN_ROLE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::clock::SystemClock;
    use crate::app::team::tests::MockTeamRepo;
    use crate::domain::team::{BanKind, Team, TeamBan};
    use chrono::{Duration, Utc};

    fn clock() -> Arc<SystemClock> {
        Arc::new(SystemClock)
    }

    #[tokio::test]
    async fn join_team_adds_member_as_observer() {
        let team = Team::new("SRE Core").unwrap();
        let code = team.invitation_code.as_str().to_string();
        let repo = Arc::new(MockTeamRepo::with_team(team.clone()));
        let user = Uuid::new_v4();
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: code,
                user_id: user,
            })
            .await
            .unwrap();

        assert_eq!(result.role, Role::Observer);
        assert_eq!(result.team_id, team.id);
        let added = repo.added.lock().unwrap();
        assert_eq!(added.as_slice(), &[(team.id, user, Role::Observer)]);
    }

    #[tokio::test]
    async fn join_team_rejects_unknown_invitation_code() {
        let repo = Arc::new(MockTeamRepo::default());
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: "OPS-ZZZZZZ".to_string(),
                user_id: Uuid::new_v4(),
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::TeamNotFound);
        assert!(repo.added.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn join_team_refuses_existing_member() {
        let team = Team::new("SRE Core").unwrap();
        let code = team.invitation_code.as_str().to_string();
        let user = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::with_team(team).with_member(user, Role::Responder));
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: code,
                user_id: user,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::AlreadyMember);
        assert!(repo.added.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_active_ban_blocks_joining() {
        let team = Team::new("SRE Core").unwrap();
        let code = team.invitation_code.as_str().to_string();
        let user = Uuid::new_v4();
        let ban = TeamBan::permanent(team.id, user, Uuid::new_v4(), None);
        let repo = Arc::new(MockTeamRepo::with_team(team).with_ban(ban));
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: code,
                user_id: user,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::UserBanned);
        assert!(repo.added.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_expired_temporary_ban_allows_joining() {
        let team = Team::new("SRE Core").unwrap();
        let code = team.invitation_code.as_str().to_string();
        let user = Uuid::new_v4();
        // Build an already-expired ban directly: the constructor rejects past
        // expiries, but a stored row can naturally age out.
        let expired = TeamBan {
            team_id: team.id,
            user_id: user,
            kind: BanKind::Temporary {
                expires_at: Utc::now() - Duration::hours(1),
            },
            reason: None,
            created_by: Some(Uuid::new_v4()),
            created_at: Utc::now() - Duration::hours(2),
        };
        let repo = Arc::new(MockTeamRepo::with_team(team.clone()).with_ban(expired));
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: code,
                user_id: user,
            })
            .await
            .unwrap();

        assert_eq!(result.role, Role::Observer);
        assert_eq!(
            repo.added.lock().unwrap().as_slice(),
            &[(team.id, user, Role::Observer)]
        );
    }

    #[tokio::test]
    async fn join_team_normalizes_code_whitespace_and_case() {
        let team = Team::new("SRE Core").unwrap();
        let raw_code = team.invitation_code.as_str().to_string();
        let dirty_code = format!("  {}  ", raw_code.to_lowercase());
        let repo = Arc::new(MockTeamRepo::with_team(team.clone()));
        let user = Uuid::new_v4();
        let use_case = JoinTeamUseCase::new(repo.clone(), clock());

        let result = use_case
            .join_team(JoinTeamCommand {
                invitation_code: dirty_code,
                user_id: user,
            })
            .await
            .unwrap();

        assert_eq!(result.role, Role::Observer);
        assert_eq!(result.team_id, team.id);
    }
}
