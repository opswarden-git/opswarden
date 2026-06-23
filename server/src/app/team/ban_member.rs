// --- server/src/app/team/ban_member.rs ---
//
// A Manager bans a user from a team. A ban is a persistent record that blocks
// (re)joining; it is independent of membership, so banning a current member also
// drops their membership. The target may be a non-member (a pre-emptive ban).
// Manager-only; self and the Manager seat cannot be banned.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::team::{validate_member_moderation, Role, TeamBan};
use crate::ports::{IncidentRepo, TeamRepo, UserRepo};

/// What the caller asked for; the use-case turns it into a validated `TeamBan`.
pub enum BanRequest {
    Temporary { expires_at: DateTime<Utc> },
    Permanent,
}

pub struct BanMemberCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub target_user_id: Uuid,
    pub request: BanRequest,
    pub reason: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BanMemberResult {
    pub user_id: Uuid,
    /// `None` for a permanent ban.
    pub expires_at: Option<DateTime<Utc>>,
    /// `true` when the target was a current member whose membership was dropped.
    pub removed_membership: bool,
}

pub struct BanMemberUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    users: Arc<dyn UserRepo>,
}

impl BanMemberUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        users: Arc<dyn UserRepo>,
    ) -> Self {
        Self {
            teams,
            incidents,
            users,
        }
    }

    pub async fn ban_member(&self, cmd: BanMemberCommand) -> Result<BanMemberResult, DomainError> {
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if requester_role != Role::Manager {
            return Err(DomainError::NotManager);
        }

        // A ban may target a non-member (pre-emptive), so the role is optional.
        let target_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.target_user_id)
            .await?;
        validate_member_moderation(cmd.requester_id, cmd.target_user_id, target_role)?;

        // A current member necessarily exists; for a pre-emptive ban of a
        // non-member, confirm the account is real so an unknown id yields a
        // clean 404 instead of a foreign-key 500.
        if target_role.is_none() && self.users.find_by_id(cmd.target_user_id).await?.is_none() {
            return Err(DomainError::UserNotFound);
        }

        let ban = match cmd.request {
            BanRequest::Temporary { expires_at } => TeamBan::temporary(
                cmd.team_id,
                cmd.target_user_id,
                cmd.requester_id,
                expires_at,
                cmd.reason,
            )?,
            BanRequest::Permanent => TeamBan::permanent(
                cmd.team_id,
                cmd.target_user_id,
                cmd.requester_id,
                cmd.reason,
            ),
        };
        let expires_at = ban.expires_at();
        self.teams.add_ban(&ban).await?;

        // Drop the membership if the banned user is currently in the team.
        let removed_membership = target_role.is_some();
        if removed_membership {
            self.teams
                .remove_member(cmd.team_id, cmd.target_user_id)
                .await?;
            // No incident may stay assigned to the now-removed member.
            self.incidents
                .clear_assignee_for_member(cmd.team_id, cmd.target_user_id)
                .await?;
        }

        Ok(BanMemberResult {
            user_id: cmd.target_user_id,
            expires_at,
            removed_membership,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::tests::MockUserRepo;
    use crate::app::incident::tests::MockIncidentRepo;
    use crate::app::team::tests::MockTeamRepo;

    // Build the use-case with fresh incident/user mocks. `user_exists` controls
    // whether a pre-emptive ban's target resolves to a real account; the returned
    // incident mock lets a test assert that assignments were cleared.
    fn build(
        teams: Arc<MockTeamRepo>,
        user_exists: bool,
    ) -> (BanMemberUseCase, Arc<MockIncidentRepo>) {
        let incidents = Arc::new(MockIncidentRepo::default());
        let users = Arc::new(MockUserRepo {
            simulate_user_exists: user_exists,
        });
        (
            BanMemberUseCase::new(teams, incidents.clone(), users),
            incidents,
        )
    }

    #[tokio::test]
    async fn manager_permanently_bans_a_member_and_drops_membership() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(observer, Role::Observer),
        );
        let (use_case, incidents) = build(repo.clone(), true);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: observer,
                request: BanRequest::Permanent,
                reason: Some("spam".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.expires_at, None);
        assert!(result.removed_membership);
        assert_eq!(repo.removed.lock().unwrap().as_slice(), &[(team, observer)]);
        // The banned member's incident assignments are cleared.
        assert_eq!(
            incidents.cleared.lock().unwrap().as_slice(),
            &[(team, observer)]
        );
        let bans = repo.bans.lock().unwrap();
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].user_id, observer);
        assert!(bans[0].is_active(Utc::now()));
    }

    #[tokio::test]
    async fn manager_temporarily_bans_a_member() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let expires = Utc::now() + chrono::Duration::hours(2);
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(responder, Role::Responder),
        );
        let (use_case, _incidents) = build(repo.clone(), true);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: responder,
                request: BanRequest::Temporary {
                    expires_at: expires,
                },
                reason: None,
            })
            .await
            .unwrap();

        assert_eq!(result.expires_at, Some(expires));
        assert!(repo.bans.lock().unwrap()[0].is_active(Utc::now()));
    }

    #[tokio::test]
    async fn a_pre_emptive_ban_of_a_non_member_records_no_removal() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let (use_case, incidents) = build(repo.clone(), true);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: stranger,
                request: BanRequest::Permanent,
                reason: None,
            })
            .await
            .unwrap();

        assert!(!result.removed_membership);
        assert!(repo.removed.lock().unwrap().is_empty());
        assert!(incidents.cleared.lock().unwrap().is_empty());
        assert_eq!(repo.bans.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn non_manager_cannot_ban() {
        let team = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(responder, Role::Responder)
                .with_member(observer, Role::Observer),
        );
        let (use_case, _incidents) = build(repo.clone(), true);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: responder,
                target_user_id: observer,
                request: BanRequest::Permanent,
                reason: None,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
        assert!(repo.bans.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn manager_cannot_ban_self_or_another_manager() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let other_manager = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(other_manager, Role::Manager),
        );
        let (use_case, _incidents) = build(repo.clone(), true);

        let self_ban = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: manager,
                request: BanRequest::Permanent,
                reason: None,
            })
            .await;
        assert_eq!(self_ban.unwrap_err(), DomainError::CannotModerateSelf);

        let manager_ban = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: other_manager,
                request: BanRequest::Permanent,
                reason: None,
            })
            .await;
        assert_eq!(manager_ban.unwrap_err(), DomainError::CannotModerateManager);
        assert!(repo.bans.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_temporary_ban_in_the_past_is_rejected() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(observer, Role::Observer),
        );
        let (use_case, _incidents) = build(repo.clone(), true);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: observer,
                request: BanRequest::Temporary {
                    expires_at: Utc::now() - chrono::Duration::hours(1),
                },
                reason: None,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::InvalidBanExpiry);
        assert!(repo.bans.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn banning_a_nonexistent_user_is_user_not_found() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let ghost = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        // The target is not a member and the account does not exist.
        let (use_case, _incidents) = build(repo.clone(), false);

        let result = use_case
            .ban_member(BanMemberCommand {
                team_id: team,
                requester_id: manager,
                target_user_id: ghost,
                request: BanRequest::Permanent,
                reason: None,
            })
            .await;

        assert_eq!(result.unwrap_err(), DomainError::UserNotFound);
        assert!(repo.bans.lock().unwrap().is_empty());
    }
}
