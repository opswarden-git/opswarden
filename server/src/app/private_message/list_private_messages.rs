// --- server/src/app/private_message/list_private_messages.rs ---
//
// Read the conversation between the requester and a peer. The requester is, by
// construction, one of the two participants, so the conversation can only ever
// be their own. We still enforce the same gate as sending — distinct peer, peer
// exists, shared team — so an unrelated user is refused rather than handed an
// empty list, and the result is scoped strictly to the (requester, peer) pair.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::private_message::PrivateMessage;
use crate::ports::{PrivateMessageRepo, TeamRepo, UserRepo};

use super::users_share_team;

pub const DEFAULT_CONVERSATION_LIMIT: u32 = 50;
pub const MAX_CONVERSATION_LIMIT: u32 = 100;

pub struct ListPrivateMessagesCommand {
    pub requester_id: Uuid,
    pub peer_id: Uuid,
    pub limit: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListPrivateMessagesResult {
    /// Newest first, capped at the (clamped) limit.
    pub messages: Vec<PrivateMessage>,
}

pub struct ListPrivateMessagesUseCase {
    users: Arc<dyn UserRepo>,
    teams: Arc<dyn TeamRepo>,
    messages: Arc<dyn PrivateMessageRepo>,
}

impl ListPrivateMessagesUseCase {
    pub fn new(
        users: Arc<dyn UserRepo>,
        teams: Arc<dyn TeamRepo>,
        messages: Arc<dyn PrivateMessageRepo>,
    ) -> Self {
        Self {
            users,
            teams,
            messages,
        }
    }

    pub async fn list(
        &self,
        cmd: ListPrivateMessagesCommand,
    ) -> Result<ListPrivateMessagesResult, DomainError> {
        if cmd.requester_id == cmd.peer_id {
            return Err(DomainError::NoSharedTeam);
        }

        self.users
            .find_by_id(cmd.peer_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        if !users_share_team(&self.teams, cmd.requester_id, cmd.peer_id).await? {
            return Err(DomainError::NoSharedTeam);
        }

        let limit = cmd
            .limit
            .unwrap_or(DEFAULT_CONVERSATION_LIMIT)
            .clamp(1, MAX_CONVERSATION_LIMIT);

        let messages = self
            .messages
            .list_conversation(cmd.requester_id, cmd.peer_id, limit)
            .await?;

        Ok(ListPrivateMessagesResult { messages })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::MockTeamRepo;
    use crate::app::private_message::tests::{MockPrivateMessageRepo, MockUserRepo};
    use crate::domain::team::Role;

    /// Seed a conversation `a -> b` then `b -> a` so both directions exist.
    fn seeded_repo(a: Uuid, b: Uuid) -> Arc<MockPrivateMessageRepo> {
        let repo = MockPrivateMessageRepo::default();
        repo.saved
            .lock()
            .unwrap()
            .push(PrivateMessage::new(a, b, "first from a").unwrap());
        repo.saved
            .lock()
            .unwrap()
            .push(PrivateMessage::new(b, a, "reply from b").unwrap());
        Arc::new(repo)
    }

    #[tokio::test]
    async fn sender_can_list_the_conversation() {
        let team = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(a).with_user(b);
        let teams = MockTeamRepo::default()
            .with_member(team, a, Role::Observer)
            .with_member(team, b, Role::Observer);
        let messages = seeded_repo(a, b);
        let uc = ListPrivateMessagesUseCase::new(Arc::new(users), Arc::new(teams), messages);

        let result = uc
            .list(ListPrivateMessagesCommand {
                requester_id: a,
                peer_id: b,
                limit: None,
            })
            .await
            .unwrap();

        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn peer_sees_the_same_conversation() {
        let team = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(a).with_user(b);
        let teams = MockTeamRepo::default()
            .with_member(team, a, Role::Observer)
            .with_member(team, b, Role::Observer);
        let messages = seeded_repo(a, b);
        let uc = ListPrivateMessagesUseCase::new(Arc::new(users), Arc::new(teams), messages);

        // b lists with peer = a: same two messages, regardless of direction.
        let result = uc
            .list(ListPrivateMessagesCommand {
                requester_id: b,
                peer_id: a,
                limit: None,
            })
            .await
            .unwrap();

        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn an_unrelated_user_cannot_list_the_conversation() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        // a and b share a team; the stranger shares none with a.
        let team_ab = Uuid::new_v4();
        let team_stranger = Uuid::new_v4();
        let users = MockUserRepo::default()
            .with_user(a)
            .with_user(b)
            .with_user(stranger);
        let teams = MockTeamRepo::default()
            .with_member(team_ab, a, Role::Observer)
            .with_member(team_ab, b, Role::Observer)
            .with_member(team_stranger, stranger, Role::Observer);
        let messages = seeded_repo(a, b);
        let uc = ListPrivateMessagesUseCase::new(Arc::new(users), Arc::new(teams), messages);

        let err = uc
            .list(ListPrivateMessagesCommand {
                requester_id: stranger,
                peer_id: a,
                limit: None,
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::NoSharedTeam);
    }

    #[tokio::test]
    async fn the_limit_is_clamped() {
        let team = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(a).with_user(b);
        let teams = MockTeamRepo::default()
            .with_member(team, a, Role::Observer)
            .with_member(team, b, Role::Observer);
        let messages = seeded_repo(a, b);
        let uc = ListPrivateMessagesUseCase::new(Arc::new(users), Arc::new(teams), messages);

        // limit 1 returns only the newest message of the two.
        let result = uc
            .list(ListPrivateMessagesCommand {
                requester_id: a,
                peer_id: b,
                limit: Some(1),
            })
            .await
            .unwrap();

        assert_eq!(result.messages.len(), 1);
    }
}
