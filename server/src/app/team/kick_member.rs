// --- server/src/app/team/kick_member.rs ---
//
// A Manager removes another member from their team. Manager-only; the Manager
// themselves (the only one, per the single-Manager invariant) and a non-member
// cannot be targeted. A kick removes membership only — it records no ban, so the
// user may rejoin via the invitation code unless they are separately banned.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::capabilities::derive_capabilities;
use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::team::validate_member_moderation;
#[cfg(test)]
use crate::domain::team::Role;
use crate::ports::{EventPublisher, IncidentRepo, TeamRepo};

pub struct KickMemberCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub target_user_id: Uuid,
}

pub struct KickMemberUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    events: Arc<dyn EventPublisher>,
}

impl KickMemberUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            incidents,
            events,
        }
    }

    pub async fn kick_member(&self, cmd: KickMemberCommand) -> Result<(), DomainError> {
        // Gate on the requester being a Manager before revealing whether the
        // target exists, so a non-manager cannot probe membership.
        let requester_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !derive_capabilities(requester_role).can_manage_members {
            return Err(DomainError::NotManager);
        }

        let target_role = self
            .teams
            .find_member_role(cmd.team_id, cmd.target_user_id)
            .await?
            .ok_or(DomainError::MemberNotFound)?;

        validate_member_moderation(cmd.requester_id, cmd.target_user_id, Some(target_role))?;

        self.teams
            .remove_member(cmd.team_id, cmd.target_user_id)
            .await?;

        // No incident may stay assigned to the removed member.
        self.incidents
            .clear_assignee_for_member(cmd.team_id, cmd.target_user_id)
            .await?;

        // Notify the team's live clients: peers refresh the roster, the removed
        // user drops the team and loses access.
        self.events
            .publish(DomainEvent::TeamMemberRemoved {
                team_id: cmd.team_id,
                user_id: cmd.target_user_id,
            })
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockEventPublisher, MockIncidentRepo};
    use crate::app::team::tests::MockTeamRepo;

    fn build(
        teams: Arc<MockTeamRepo>,
    ) -> (
        KickMemberUseCase,
        Arc<MockIncidentRepo>,
        Arc<MockEventPublisher>,
    ) {
        let incidents = Arc::new(MockIncidentRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        (
            KickMemberUseCase::new(teams, incidents.clone(), events.clone()),
            incidents,
            events,
        )
    }

    fn cmd(team: Uuid, requester: Uuid, target: Uuid) -> KickMemberCommand {
        KickMemberCommand {
            team_id: team,
            requester_id: requester,
            target_user_id: target,
        }
    }

    #[tokio::test]
    async fn manager_kicks_an_observer() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(observer, Role::Observer),
        );
        let (use_case, incidents, events) = build(repo.clone());

        use_case
            .kick_member(cmd(team, manager, observer))
            .await
            .unwrap();

        assert_eq!(repo.removed.lock().unwrap().as_slice(), &[(team, observer)]);
        // The kicked member's assignments are cleared.
        assert_eq!(
            incidents.cleared.lock().unwrap().as_slice(),
            &[(team, observer)]
        );
        // And the team's live clients are notified.
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::TeamMemberRemoved { team_id, user_id }]
                if *team_id == team && *user_id == observer
        ));
    }

    #[tokio::test]
    async fn manager_kicks_a_responder() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(responder, Role::Responder),
        );
        let (use_case, _incidents, _events) = build(repo.clone());

        use_case
            .kick_member(cmd(team, manager, responder))
            .await
            .unwrap();

        assert_eq!(
            repo.removed.lock().unwrap().as_slice(),
            &[(team, responder)]
        );
    }

    #[tokio::test]
    async fn non_manager_cannot_kick() {
        let team = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(responder, Role::Responder)
                .with_member(observer, Role::Observer),
        );
        let (use_case, _incidents, _events) = build(repo.clone());

        let result = use_case.kick_member(cmd(team, responder, observer)).await;

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
        assert!(repo.removed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn manager_cannot_kick_self() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let (use_case, _incidents, _events) = build(repo.clone());

        let result = use_case.kick_member(cmd(team, manager, manager)).await;

        assert_eq!(result.unwrap_err(), DomainError::CannotModerateSelf);
        assert!(repo.removed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_manager_target_is_protected() {
        // Two Managers is impossible in production (single-Manager invariant) but
        // the rule is enforced defensively regardless.
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let other_manager = Uuid::new_v4();
        let repo = Arc::new(
            MockTeamRepo::default()
                .with_member(manager, Role::Manager)
                .with_member(other_manager, Role::Manager),
        );
        let (use_case, _incidents, _events) = build(repo.clone());

        let result = use_case
            .kick_member(cmd(team, manager, other_manager))
            .await;

        assert_eq!(result.unwrap_err(), DomainError::CannotModerateManager);
        assert!(repo.removed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unknown_target_is_member_not_found() {
        let team = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let repo = Arc::new(MockTeamRepo::default().with_member(manager, Role::Manager));
        let (use_case, _incidents, _events) = build(repo.clone());

        let result = use_case
            .kick_member(cmd(team, manager, Uuid::new_v4()))
            .await;

        assert_eq!(result.unwrap_err(), DomainError::MemberNotFound);
        assert!(repo.removed.lock().unwrap().is_empty());
    }
}
