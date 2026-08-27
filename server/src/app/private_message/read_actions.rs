// --- server/src/app/private_message/read_actions.rs ---

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use super::users_share_team;
use crate::domain::error::DomainError;
use crate::ports::{PrivateMessageRepo, TeamRepo};

pub struct MarkPrivateMessageReadCommand {
    pub viewer_id: Uuid,
    pub peer_id: Uuid,
    pub read_through: DateTime<Utc>,
}

pub struct MarkPrivateMessageReadUseCase {
    teams: Arc<dyn TeamRepo>,
    private_messages: Arc<dyn PrivateMessageRepo>,
}

impl MarkPrivateMessageReadUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, private_messages: Arc<dyn PrivateMessageRepo>) -> Self {
        Self {
            teams,
            private_messages,
        }
    }

    pub async fn execute(&self, cmd: MarkPrivateMessageReadCommand) -> Result<(), DomainError> {
        if cmd.viewer_id == cmd.peer_id {
            return Err(DomainError::Forbidden);
        }
        if !users_share_team(&*self.teams, cmd.viewer_id, cmd.peer_id).await? {
            return Err(DomainError::Forbidden);
        }
        self.private_messages
            .mark_read(cmd.viewer_id, cmd.peer_id, cmd.read_through)
            .await
    }
}

pub struct ListUnreadPrivateMessagesUseCase {
    private_messages: Arc<dyn PrivateMessageRepo>,
}

impl ListUnreadPrivateMessagesUseCase {
    pub fn new(private_messages: Arc<dyn PrivateMessageRepo>) -> Self {
        Self { private_messages }
    }

    pub async fn execute(&self, viewer_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        self.private_messages.list_unread_peer_ids(viewer_id).await
    }
}
