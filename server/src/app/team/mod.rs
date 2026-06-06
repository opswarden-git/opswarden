// --- server/src/app/team/mod.rs ---
pub mod create_team;
pub mod join_team;
pub mod transfer_manager;

pub use create_team::{CreateTeamCommand, CreateTeamResult, CreateTeamUseCase};
pub use join_team::{JoinTeamCommand, JoinTeamResult, JoinTeamUseCase};
pub use transfer_manager::{TransferManagerCommand, TransferManagerResult, TransferManagerUseCase};

// Shared in-memory mock for the team use-case tests (no DB in this run).
#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::team::{Role, Team};
    use crate::ports::TeamRepo;

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
    }
}
