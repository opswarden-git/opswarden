// --- server/src/app/private_message/send_private_message.rs ---
//
// Send a private message to another team member. Authorization order: a PM needs
// a distinct peer (no self-messaging), the recipient must exist, and the two
// users must share at least one team. Only then is the (length-validated) body
// persisted and the `private_message_received` event fanned out to exactly the
// two participants.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::private_message::PrivateMessage;
use crate::ports::{EventPublisher, PrivateMessageRepo, TeamRepo, UserRepo};

use super::users_share_team;

pub struct SendPrivateMessageCommand {
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SendPrivateMessageResult {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub struct SendPrivateMessageUseCase {
    users: Arc<dyn UserRepo>,
    teams: Arc<dyn TeamRepo>,
    messages: Arc<dyn PrivateMessageRepo>,
    events: Arc<dyn EventPublisher>,
}

impl SendPrivateMessageUseCase {
    pub fn new(
        users: Arc<dyn UserRepo>,
        teams: Arc<dyn TeamRepo>,
        messages: Arc<dyn PrivateMessageRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            users,
            teams,
            messages,
            events,
        }
    }

    pub async fn send(
        &self,
        cmd: SendPrivateMessageCommand,
    ) -> Result<SendPrivateMessageResult, DomainError> {
        // A private message is strictly between two *distinct* users.
        if cmd.sender_id == cmd.recipient_id {
            return Err(DomainError::NoSharedTeam);
        }

        // The recipient must be a real account...
        self.users
            .find_by_id(cmd.recipient_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        // ...and the two users must share at least one team.
        if !users_share_team(&self.teams, cmd.sender_id, cmd.recipient_id).await? {
            return Err(DomainError::NoSharedTeam);
        }

        let message = PrivateMessage::new(cmd.sender_id, cmd.recipient_id, cmd.content)?;
        self.messages.save(&message).await?;

        self.events
            .publish(DomainEvent::PrivateMessageReceived {
                message_id: message.id,
                sender_id: message.sender_id,
                recipient_id: message.recipient_id,
                content: message.content.clone(),
                at: message.created_at,
            })
            .await;

        Ok(SendPrivateMessageResult {
            message_id: message.id,
            sender_id: message.sender_id,
            recipient_id: message.recipient_id,
            content: message.content,
            created_at: message.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::incident::tests::{MockEventPublisher, MockTeamRepo};
    use crate::app::private_message::tests::{MockPrivateMessageRepo, MockUserRepo};
    use crate::domain::team::Role;

    fn use_case(
        users: MockUserRepo,
        teams: MockTeamRepo,
        messages: Arc<MockPrivateMessageRepo>,
        events: Arc<MockEventPublisher>,
    ) -> SendPrivateMessageUseCase {
        SendPrivateMessageUseCase::new(Arc::new(users), Arc::new(teams), messages, events)
    }

    #[tokio::test]
    async fn shared_team_members_can_send() {
        let team = Uuid::new_v4();
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(recipient);
        let teams = MockTeamRepo::default()
            .with_member(team, sender, Role::Observer)
            .with_member(team, recipient, Role::Observer);
        let messages = Arc::new(MockPrivateMessageRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(users, teams, messages.clone(), events.clone());

        let result = uc
            .send(SendPrivateMessageCommand {
                sender_id: sender,
                recipient_id: recipient,
                content: "  ready when you are  ".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(result.sender_id, sender);
        assert_eq!(result.recipient_id, recipient);
        assert_eq!(result.content, "ready when you are");
        assert_eq!(messages.saved.lock().unwrap().len(), 1);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::PrivateMessageReceived { .. }]
        ));
    }

    #[tokio::test]
    async fn members_without_a_shared_team_are_forbidden() {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        // Recipient exists, but each user is in a different team.
        let users = MockUserRepo::default().with_user(recipient);
        let teams = MockTeamRepo::default()
            .with_member(Uuid::new_v4(), sender, Role::Observer)
            .with_member(Uuid::new_v4(), recipient, Role::Observer);
        let messages = Arc::new(MockPrivateMessageRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(users, teams, messages.clone(), events.clone());

        let err = uc
            .send(SendPrivateMessageCommand {
                sender_id: sender,
                recipient_id: recipient,
                content: "hello?".to_string(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::NoSharedTeam);
        assert!(messages.saved.lock().unwrap().is_empty());
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_missing_recipient_is_user_not_found() {
        let team = Uuid::new_v4();
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        // Recipient is NOT registered in the user repo.
        let users = MockUserRepo::default();
        let teams = MockTeamRepo::default()
            .with_member(team, sender, Role::Observer)
            .with_member(team, recipient, Role::Observer);
        let messages = Arc::new(MockPrivateMessageRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(users, teams, messages.clone(), events.clone());

        let err = uc
            .send(SendPrivateMessageCommand {
                sender_id: sender,
                recipient_id: recipient,
                content: "anyone there?".to_string(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::UserNotFound);
        assert!(messages.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn messaging_yourself_is_rejected() {
        let team = Uuid::new_v4();
        let me = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(me);
        let teams = MockTeamRepo::default().with_member(team, me, Role::Observer);
        let messages = Arc::new(MockPrivateMessageRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(users, teams, messages.clone(), events.clone());

        let err = uc
            .send(SendPrivateMessageCommand {
                sender_id: me,
                recipient_id: me,
                content: "note to self".to_string(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::NoSharedTeam);
        assert!(messages.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_blank_message_is_rejected_after_authorization() {
        let team = Uuid::new_v4();
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let users = MockUserRepo::default().with_user(recipient);
        let teams = MockTeamRepo::default()
            .with_member(team, sender, Role::Observer)
            .with_member(team, recipient, Role::Observer);
        let messages = Arc::new(MockPrivateMessageRepo::default());
        let events = Arc::new(MockEventPublisher::default());
        let uc = use_case(users, teams, messages.clone(), events.clone());

        let err = uc
            .send(SendPrivateMessageCommand {
                sender_id: sender,
                recipient_id: recipient,
                content: "   ".to_string(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::InvalidPrivateMessage);
        assert!(messages.saved.lock().unwrap().is_empty());
        assert!(events.published.lock().unwrap().is_empty());
    }
}
