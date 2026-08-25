// --- server/src/app/private_message/mod.rs ---
//
// Private messaging use-cases. A private message is bilateral and
// 1-to-1, not attached to any incident/release/team. The only authorization
// rule is "the two users share at least one team"; it is enforced here, in the
// app layer, by intersecting their team memberships through the existing
// `TeamRepo`. No new repo method is needed for the check.

use std::collections::HashSet;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::ports::TeamRepo;

pub mod list_private_messages;
pub mod message_actions;
pub mod send_private_message;

pub use list_private_messages::{
    ListPrivateMessagesCommand, ListPrivateMessagesResult, ListPrivateMessagesUseCase,
    DEFAULT_CONVERSATION_LIMIT, MAX_CONVERSATION_LIMIT,
};
pub use message_actions::{
    EditPrivateMessageCommand, EditPrivateMessageUseCase, GetPrivateMessageAttachmentUseCase,
    TogglePrivateMessageReactionCommand, TogglePrivateMessageReactionUseCase,
};
pub use send_private_message::{
    SendPrivateMessageCommand, SendPrivateMessageResult, SendPrivateMessageUseCase,
};

/// Whether two users share at least one team. Reuses `list_team_ids_for_user`
/// (already needed by the WebSocket hub) rather than adding a bespoke repo
/// method: at this scale intersecting two small id sets is cheap and keeps the
/// port surface unchanged.
pub(crate) async fn users_share_team(
    teams: &dyn TeamRepo,
    a: Uuid,
    b: Uuid,
) -> Result<bool, DomainError> {
    let a_teams: HashSet<Uuid> = teams.list_team_ids_for_user(a).await?.into_iter().collect();
    if a_teams.is_empty() {
        return Ok(false);
    }
    let b_teams = teams.list_team_ids_for_user(b).await?;
    Ok(b_teams.iter().any(|team_id| a_teams.contains(team_id)))
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::private_message::{
        PrivateMessage, PrivateMessageAttachment, PrivateMessageReaction,
    };
    use crate::domain::user::{Email, Locale, User};
    use crate::ports::{PrivateMessageRepo, UserRepo};

    /// A user directory keyed by id. `find_by_id` returns a synthetic user for
    /// any seeded id, so the use-cases can assert the recipient-exists branch.
    #[derive(Default)]
    pub struct MockUserRepo {
        ids: HashSet<Uuid>,
    }

    impl MockUserRepo {
        pub fn with_user(mut self, id: Uuid) -> Self {
            self.ids.insert(id);
            self
        }
    }

    #[async_trait]
    impl UserRepo for MockUserRepo {
        async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError> {
            if self.ids.contains(&user_id) {
                let email = Email::new(format!("user-{user_id}@test.local")).unwrap();
                Ok(Some(User {
                    id: user_id,
                    email,
                    password_hash: "hash".to_string(),
                    locale: Locale::En,
                    created_at: Utc::now(),
                }))
            } else {
                Ok(None)
            }
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
            Ok(None)
        }

        async fn save(&self, _user: &User) -> Result<(), DomainError> {
            Ok(())
        }

        async fn update_locale(&self, _user_id: Uuid, _locale: Locale) -> Result<(), DomainError> {
            Ok(())
        }

        async fn delete_account(&self, _user_id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct MockPrivateMessageRepo {
        pub saved: Mutex<Vec<PrivateMessage>>,
        reactions: Mutex<HashSet<(Uuid, Uuid, String)>>,
    }

    #[async_trait]
    impl PrivateMessageRepo for MockPrivateMessageRepo {
        async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError> {
            self.saved.lock().unwrap().push(message.clone());
            Ok(())
        }

        async fn list_conversation(
            &self,
            viewer_id: Uuid,
            peer_id: Uuid,
            before: Option<(DateTime<Utc>, Uuid)>,
            limit: u32,
        ) -> Result<Vec<PrivateMessage>, DomainError> {
            let reactions = self.reactions.lock().unwrap();
            let mut msgs: Vec<PrivateMessage> = self
                .saved
                .lock()
                .unwrap()
                .iter()
                .filter(|m| {
                    ((m.sender_id == viewer_id && m.recipient_id == peer_id)
                        || (m.sender_id == peer_id && m.recipient_id == viewer_id))
                        && before.is_none_or(|cursor| (m.created_at, m.id) < cursor)
                })
                .cloned()
                .collect();
            for message in &mut msgs {
                let mut by_emoji: HashMap<String, (u64, bool)> = HashMap::new();
                for (_, user_id, emoji) in reactions.iter().filter(|(id, _, _)| *id == message.id) {
                    let summary = by_emoji.entry(emoji.clone()).or_default();
                    summary.0 += 1;
                    summary.1 |= *user_id == viewer_id;
                }
                message.reactions = by_emoji
                    .into_iter()
                    .map(|(emoji, (count, reacted))| PrivateMessageReaction {
                        emoji,
                        count,
                        reacted,
                    })
                    .collect();
            }
            // Newest first, matching the PG adapter's ORDER BY created_at DESC.
            msgs.sort_by_key(|m| std::cmp::Reverse(m.created_at));
            msgs.truncate(limit as usize);
            Ok(msgs)
        }

        async fn find_participants(
            &self,
            message_id: Uuid,
        ) -> Result<Option<(Uuid, Uuid)>, DomainError> {
            Ok(self
                .saved
                .lock()
                .unwrap()
                .iter()
                .find(|message| message.id == message_id)
                .map(|message| (message.sender_id, message.recipient_id)))
        }

        async fn update_content(
            &self,
            message_id: Uuid,
            content: &str,
            edited_at: DateTime<Utc>,
        ) -> Result<(), DomainError> {
            if let Some(message) = self
                .saved
                .lock()
                .unwrap()
                .iter_mut()
                .find(|message| message.id == message_id)
            {
                message.content = content.to_string();
                message.edited_at = Some(edited_at);
            }
            Ok(())
        }

        async fn toggle_reaction(
            &self,
            message_id: Uuid,
            user_id: Uuid,
            emoji: &str,
        ) -> Result<bool, DomainError> {
            let key = (message_id, user_id, emoji.to_string());
            let mut reactions = self.reactions.lock().unwrap();
            if reactions.remove(&key) {
                Ok(false)
            } else {
                reactions.insert(key);
                Ok(true)
            }
        }

        async fn find_attachment_for_participant(
            &self,
            attachment_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<PrivateMessageAttachment>, DomainError> {
            Ok(self
                .saved
                .lock()
                .unwrap()
                .iter()
                .filter(|message| message.sender_id == user_id || message.recipient_id == user_id)
                .flat_map(|message| message.attachments.iter())
                .find(|attachment| attachment.id == attachment_id)
                .cloned())
        }
    }
}
