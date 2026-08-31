// Shared in-memory team and event doubles for the incident use cases.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::team::{Role, TeamBanView, TeamDirectoryItem, TeamMemberView};
use crate::ports::{EventPublisher, TeamRepo};

#[derive(Default)]
pub struct MockTeamRepo {
    pub roles: HashMap<(Uuid, Uuid), Role>,
}

impl MockTeamRepo {
    pub fn with_member(mut self, team_id: Uuid, user_id: Uuid, role: Role) -> Self {
        self.roles.insert((team_id, user_id), role);
        self
    }
}

#[async_trait]
impl TeamRepo for MockTeamRepo {
    async fn create_team_with_manager(
        &self,
        _team: &crate::domain::team::Team,
        _manager_id: Uuid,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn find_by_invitation_code(
        &self,
        _code: &str,
    ) -> Result<Option<crate::domain::team::Team>, DomainError> {
        Ok(None)
    }

    async fn find_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, DomainError> {
        Ok(self.roles.get(&(team_id, user_id)).copied())
    }

    async fn add_member(
        &self,
        _team_id: Uuid,
        _user_id: Uuid,
        _role: Role,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn transfer_manager(
        &self,
        _team_id: Uuid,
        _old_manager: Uuid,
        _new_manager: Uuid,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        Ok(self
            .roles
            .keys()
            .filter(|(_, u)| *u == user_id)
            .map(|(t, _)| *t)
            .collect())
    }

    async fn list_teams_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(crate::domain::team::Team, Role)>, DomainError> {
        use crate::domain::team::{InvitationCode, Team};
        Ok(self
            .roles
            .iter()
            .filter(|((_, u), _)| *u == user_id)
            .map(|((t, _), role)| {
                (
                    Team {
                        id: *t,
                        name: format!("team-{t}"),
                        invitation_code: InvitationCode::from_existing("OPS-TEST00"),
                        created_at: Utc::now(),
                    },
                    *role,
                )
            })
            .collect())
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
                image_updated_at: None,
            })
            .collect())
    }

    async fn find_team_by_id(
        &self,
        _team_id: Uuid,
    ) -> Result<Option<crate::domain::team::Team>, DomainError> {
        Ok(None)
    }

    async fn delete_team(&self, _team_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn remove_member(&self, _team_id: Uuid, _user_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn kick_member_and_clear_assignments(
        &self,
        _team_id: Uuid,
        _requester_id: Uuid,
        _target_user_id: Uuid,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError> {
        Ok(self.roles.keys().filter(|(t, _)| *t == team_id).count() as u64)
    }

    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError> {
        Ok(self
            .roles
            .iter()
            .filter(|((t, _), _)| *t == team_id)
            .map(|((_, user_id), role)| TeamMemberView {
                user_id: *user_id,
                email: format!("user-{user_id}@test.local"),
                role: *role,
                joined_at: Utc::now(),
            })
            .collect())
    }

    async fn set_member_role(
        &self,
        _team_id: Uuid,
        _user_id: Uuid,
        _role: Role,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn add_ban(&self, _ban: &crate::domain::team::TeamBan) -> Result<(), DomainError> {
        Ok(())
    }

    async fn ban_member_and_clear_assignments(
        &self,
        _ban: &crate::domain::team::TeamBan,
        _requester_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn find_ban(
        &self,
        _team_id: Uuid,
        _user_id: Uuid,
    ) -> Result<Option<crate::domain::team::TeamBan>, DomainError> {
        Ok(None)
    }

    async fn list_bans(&self, _team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError> {
        Ok(Vec::new())
    }

    async fn remove_ban(&self, _team_id: Uuid, _user_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn save_team_image(
        &self,
        _team_id: Uuid,
        _image: &crate::domain::team::TeamImage,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn find_team_image_for_member(
        &self,
        _team_id: Uuid,
        _user_id: Uuid,
    ) -> Result<Option<crate::domain::team::TeamImage>, DomainError> {
        Ok(None)
    }

    async fn delete_team_image(&self, _team_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct MockEventPublisher {
    pub published: Mutex<Vec<DomainEvent>>,
}

#[async_trait]
impl EventPublisher for MockEventPublisher {
    async fn publish(&self, event: DomainEvent) {
        self.published.lock().unwrap().push(event);
    }
}

mod doubles;
pub use doubles::*;
