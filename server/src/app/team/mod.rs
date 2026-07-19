// --- server/src/app/team/mod.rs ---
pub mod ban_member;
pub mod create_team;
pub mod delete_team;
pub mod get_invitation_code;
pub mod join_team;
pub mod kick_member;
pub mod leave_team;
pub mod list_bans;
pub mod list_members;
pub mod list_teams;
pub mod set_member_role;
pub mod transfer_manager;
pub mod unban_member;

pub use ban_member::{BanMemberCommand, BanMemberResult, BanMemberUseCase, BanRequest};
pub use create_team::{CreateTeamCommand, CreateTeamResult, CreateTeamUseCase};
pub use delete_team::{DeleteTeamCommand, DeleteTeamUseCase};
pub use get_invitation_code::{
    GetInvitationCodeCommand, GetInvitationCodeResult, GetInvitationCodeUseCase,
};
pub use join_team::{JoinTeamCommand, JoinTeamResult, JoinTeamUseCase};
pub use kick_member::{KickMemberCommand, KickMemberUseCase};
pub use leave_team::{LeaveTeamCommand, LeaveTeamUseCase};
pub use list_bans::{ListBansCommand, ListBansResult, ListBansUseCase};
pub use list_members::{ListTeamMembersCommand, ListTeamMembersResult, ListTeamMembersUseCase};
pub use list_teams::{ListTeamsCommand, ListTeamsResult, ListTeamsUseCase, TeamSummary};
pub use set_member_role::{SetMemberRoleCommand, SetMemberRoleUseCase};
pub use transfer_manager::{TransferManagerCommand, TransferManagerResult, TransferManagerUseCase};
pub use unban_member::{UnbanMemberCommand, UnbanMemberUseCase};

// Shared in-memory mock for the team use-case tests (no DB in this run).
#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::team::{
        Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamMemberView,
    };
    use crate::ports::TeamRepo;
    use chrono::Utc;

    /// Configurable fake `TeamRepo`. Reads (`team`, `roles`) are preset via the
    /// builder helpers; writes are recorded so tests can assert on them.
    #[derive(Default)]
    pub struct MockTeamRepo {
        /// Returned by `find_by_invitation_code` when its code matches.
        pub team: Option<Team>,
        /// Preset memberships consulted by `find_member_role` (user → role).
        pub roles: HashMap<Uuid, Role>,
        pub saved: Mutex<Vec<Team>>,
        pub added: Mutex<Vec<(Uuid, Uuid, Role)>>,
        pub transfers: Mutex<Vec<(Uuid, Uuid, Uuid)>>,
        pub deleted: Mutex<Vec<Uuid>>,
        pub removed: Mutex<Vec<(Uuid, Uuid)>>,
        pub role_changes: Mutex<Vec<(Uuid, Uuid, Role)>>,
        pub bans: Mutex<Vec<TeamBan>>,
    }

    impl MockTeamRepo {
        pub fn with_team(team: Team) -> Self {
            Self {
                team: Some(team),
                ..Self::default()
            }
        }

        pub fn with_member(mut self, user_id: Uuid, role: Role) -> Self {
            self.roles.insert(user_id, role);
            self
        }

        pub fn with_ban(self, ban: TeamBan) -> Self {
            self.bans.lock().unwrap().push(ban);
            self
        }
    }

    #[async_trait]
    impl TeamRepo for MockTeamRepo {
        async fn save_team(&self, team: &Team) -> Result<(), DomainError> {
            self.saved.lock().unwrap().push(team.clone());
            Ok(())
        }

        async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError> {
            Ok(self
                .team
                .clone()
                .filter(|t| t.invitation_code.as_str() == code))
        }

        async fn find_member_role(
            &self,
            _team_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<Role>, DomainError> {
            Ok(self.roles.get(&user_id).copied())
        }

        async fn add_member(
            &self,
            team_id: Uuid,
            user_id: Uuid,
            role: Role,
        ) -> Result<(), DomainError> {
            self.added.lock().unwrap().push((team_id, user_id, role));
            Ok(())
        }

        async fn transfer_manager(
            &self,
            team_id: Uuid,
            old_manager: Uuid,
            new_manager: Uuid,
        ) -> Result<(), DomainError> {
            self.transfers
                .lock()
                .unwrap()
                .push((team_id, old_manager, new_manager));
            Ok(())
        }

        async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
            Ok(match (&self.team, self.roles.contains_key(&user_id)) {
                (Some(team), true) => vec![team.id],
                _ => vec![],
            })
        }

        async fn list_teams_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<(Team, Role)>, DomainError> {
            Ok(match (&self.team, self.roles.get(&user_id)) {
                (Some(team), Some(role)) => vec![(team.clone(), *role)],
                _ => vec![],
            })
        }

        async fn list_team_directory_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<TeamDirectoryItem>, DomainError> {
            Ok(self
                .list_teams_for_user(user_id)
                .await?
                .into_iter()
                .map(|(team, role)| TeamDirectoryItem {
                    team,
                    role,
                    member_count: self.roles.len() as u64,
                    active_incident_count: 0,
                    active_release_count: 0,
                    blocked_release_count: 0,
                })
                .collect())
        }

        async fn find_team_by_id(&self, team_id: Uuid) -> Result<Option<Team>, DomainError> {
            Ok(self.team.clone().filter(|team| team.id == team_id))
        }

        async fn delete_team(&self, team_id: Uuid) -> Result<(), DomainError> {
            self.deleted.lock().unwrap().push(team_id);
            Ok(())
        }

        async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
            self.removed.lock().unwrap().push((team_id, user_id));
            Ok(())
        }

        async fn count_members(&self, _team_id: Uuid) -> Result<u64, DomainError> {
            Ok(self.roles.len() as u64)
        }

        async fn list_members(&self, _team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError> {
            Ok(self
                .roles
                .iter()
                .map(|(user_id, role)| TeamMemberView {
                    user_id: *user_id,
                    email: format!("user-{user_id}@test.local"),
                    role: *role,
                    joined_at: Utc::now(),
                })
                .collect())
        }

        async fn set_member_role(
            &self,
            team_id: Uuid,
            user_id: Uuid,
            role: Role,
        ) -> Result<(), DomainError> {
            self.role_changes
                .lock()
                .unwrap()
                .push((team_id, user_id, role));
            Ok(())
        }

        async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError> {
            let mut bans = self.bans.lock().unwrap();
            // Upsert by user (the mock is single-team), mirroring the PG adapter.
            bans.retain(|b| b.user_id != ban.user_id);
            bans.push(ban.clone());
            Ok(())
        }

        async fn find_ban(
            &self,
            _team_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<TeamBan>, DomainError> {
            Ok(self
                .bans
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.user_id == user_id)
                .cloned())
        }

        async fn list_bans(&self, _team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
            Ok(self
                .bans
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .map(|ban| TeamBanView {
                    user_email: format!("user-{}@test.local", ban.user_id),
                    moderator_email: ban.created_by.map(|id| format!("user-{id}@test.local")),
                    ban,
                })
                .collect())
        }

        async fn remove_ban(&self, _team_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
            self.bans
                .lock()
                .unwrap()
                .retain(|ban| ban.user_id != user_id);
            Ok(())
        }
    }
}
