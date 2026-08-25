use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::private_message::{PrivateMessage, PrivateMessageAttachment};
use crate::domain::timeline::validate_reaction_emoji;
use crate::ports::{EventPublisher, PrivateMessageRepo, TeamRepo};

use super::users_share_team;

async fn authorized_participants(
    messages: &Arc<dyn PrivateMessageRepo>,
    teams: &Arc<dyn TeamRepo>,
    message_id: Uuid,
    user_id: Uuid,
) -> Result<(Uuid, Uuid), DomainError> {
    let (sender_id, recipient_id) = messages
        .find_participants(message_id)
        .await?
        .ok_or(DomainError::Forbidden)?;
    if user_id != sender_id && user_id != recipient_id {
        return Err(DomainError::Forbidden);
    }
    if !users_share_team(teams.as_ref(), sender_id, recipient_id).await? {
        return Err(DomainError::NoSharedTeam);
    }
    Ok((sender_id, recipient_id))
}

pub struct EditPrivateMessageCommand {
    pub requester_id: Uuid,
    pub message_id: Uuid,
    pub content: String,
}

#[derive(Debug)]
pub struct EditPrivateMessageResult {
    pub content: String,
    pub edited_at: DateTime<Utc>,
}

pub struct EditPrivateMessageUseCase {
    teams: Arc<dyn TeamRepo>,
    messages: Arc<dyn PrivateMessageRepo>,
    events: Arc<dyn EventPublisher>,
}

impl EditPrivateMessageUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        messages: Arc<dyn PrivateMessageRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            messages,
            events,
        }
    }

    pub async fn edit(
        &self,
        command: EditPrivateMessageCommand,
    ) -> Result<EditPrivateMessageResult, DomainError> {
        let (sender_id, recipient_id) = authorized_participants(
            &self.messages,
            &self.teams,
            command.message_id,
            command.requester_id,
        )
        .await?;
        if command.requester_id != sender_id {
            return Err(DomainError::Forbidden);
        }
        let content = PrivateMessage::validate_edited_content(command.content)?;
        let edited_at = Utc::now();
        self.messages
            .update_content(command.message_id, &content, edited_at)
            .await?;
        self.events
            .publish(DomainEvent::PrivateMessageEdited {
                message_id: command.message_id,
                sender_id,
                recipient_id,
                at: edited_at,
            })
            .await;
        Ok(EditPrivateMessageResult { content, edited_at })
    }
}

pub struct TogglePrivateMessageReactionCommand {
    pub requester_id: Uuid,
    pub message_id: Uuid,
    pub emoji: String,
}

pub struct TogglePrivateMessageReactionResult {
    pub active: bool,
}

pub struct TogglePrivateMessageReactionUseCase {
    teams: Arc<dyn TeamRepo>,
    messages: Arc<dyn PrivateMessageRepo>,
    events: Arc<dyn EventPublisher>,
}

impl TogglePrivateMessageReactionUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        messages: Arc<dyn PrivateMessageRepo>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            teams,
            messages,
            events,
        }
    }

    pub async fn toggle(
        &self,
        command: TogglePrivateMessageReactionCommand,
    ) -> Result<TogglePrivateMessageReactionResult, DomainError> {
        let emoji = validate_reaction_emoji(&command.emoji)?;
        let (sender_id, recipient_id) = authorized_participants(
            &self.messages,
            &self.teams,
            command.message_id,
            command.requester_id,
        )
        .await?;
        let active = self
            .messages
            .toggle_reaction(command.message_id, command.requester_id, &emoji)
            .await?;
        self.events
            .publish(DomainEvent::PrivateMessageReactionChanged {
                message_id: command.message_id,
                sender_id,
                recipient_id,
                emoji,
                user_id: command.requester_id,
                active,
            })
            .await;
        Ok(TogglePrivateMessageReactionResult { active })
    }
}

pub struct GetPrivateMessageAttachmentUseCase {
    teams: Arc<dyn TeamRepo>,
    messages: Arc<dyn PrivateMessageRepo>,
}

impl GetPrivateMessageAttachmentUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, messages: Arc<dyn PrivateMessageRepo>) -> Self {
        Self { teams, messages }
    }

    pub async fn get(
        &self,
        attachment_id: Uuid,
        requester_id: Uuid,
    ) -> Result<PrivateMessageAttachment, DomainError> {
        let attachment = self
            .messages
            .find_attachment_for_participant(attachment_id, requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        let (sender_id, recipient_id) = self
            .messages
            .find_participants(attachment.message_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        if !users_share_team(self.teams.as_ref(), sender_id, recipient_id).await? {
            return Err(DomainError::NoSharedTeam);
        }
        Ok(attachment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockEventPublisher, MockTeamRepo};
    use crate::app::private_message::tests::MockPrivateMessageRepo;
    use crate::domain::team::Role;

    fn setup() -> (
        Uuid,
        Uuid,
        Arc<MockPrivateMessageRepo>,
        Arc<MockTeamRepo>,
        Arc<MockEventPublisher>,
        PrivateMessage,
    ) {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let team = Uuid::new_v4();
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team, sender, Role::Observer)
                .with_member(team, recipient, Role::Observer),
        );
        let message = PrivateMessage::new_with_attachments(
            sender,
            recipient,
            "hello",
            vec![("runbook.pdf".into(), "application/pdf".into(), vec![1, 2])],
        )
        .unwrap();
        let messages = Arc::new(MockPrivateMessageRepo::default());
        messages.saved.lock().unwrap().push(message.clone());
        let events = Arc::new(MockEventPublisher::default());
        (sender, recipient, messages, teams, events, message)
    }

    #[tokio::test]
    async fn author_edits_and_both_clients_receive_an_event() {
        let (sender, _, messages, teams, events, message) = setup();
        let result = EditPrivateMessageUseCase::new(teams, messages.clone(), events.clone())
            .edit(EditPrivateMessageCommand {
                requester_id: sender,
                message_id: message.id,
                content: " updated ".into(),
            })
            .await
            .unwrap();
        assert_eq!(result.content, "updated");
        assert_eq!(messages.saved.lock().unwrap()[0].content, "updated");
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::PrivateMessageEdited { .. }]
        ));
    }

    #[tokio::test]
    async fn recipient_cannot_edit_the_authors_message() {
        let (_, recipient, messages, teams, events, message) = setup();
        let error = EditPrivateMessageUseCase::new(teams, messages, events)
            .edit(EditPrivateMessageCommand {
                requester_id: recipient,
                message_id: message.id,
                content: "tamper".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(error, DomainError::Forbidden);
    }

    #[tokio::test]
    async fn participant_toggles_a_valid_reaction() {
        let (_, recipient, messages, teams, events, message) = setup();
        let result = TogglePrivateMessageReactionUseCase::new(teams, messages, events.clone())
            .toggle(TogglePrivateMessageReactionCommand {
                requester_id: recipient,
                message_id: message.id,
                emoji: " ✅ ".into(),
            })
            .await
            .unwrap();
        assert!(result.active);
        assert!(matches!(
            events.published.lock().unwrap().as_slice(),
            [DomainEvent::PrivateMessageReactionChanged { active: true, .. }]
        ));
    }

    #[tokio::test]
    async fn participant_downloads_the_bounded_attachment() {
        let (_, recipient, messages, teams, _, message) = setup();
        let attachment = GetPrivateMessageAttachmentUseCase::new(teams, messages)
            .get(message.attachments[0].id, recipient)
            .await
            .unwrap();
        assert_eq!(attachment.file_name, "runbook.pdf");
        assert_eq!(attachment.content, vec![1, 2]);
    }
}
