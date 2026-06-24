// --- server/src/adapters/pg/private_message.rs ---
//
// Postgres adapter for private messages. Reads fetch both directions of the
// participant pair (the conversation is symmetric) newest-first.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::private_message::PrivateMessage;
use crate::ports::PrivateMessageRepo;

pub struct PgPrivateMessageRepo {
    pool: PgPool,
}

impl PgPrivateMessageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrivateMessageRepo for PgPrivateMessageRepo {
    async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO private_messages (id, sender_id, recipient_id, content, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            message.id,
            message.sender_id,
            message.recipient_id,
            message.content,
            message.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(())
    }

    async fn list_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError> {
        let records = sqlx::query!(
            r#"
            SELECT id, sender_id, recipient_id, content, created_at
            FROM private_messages
            WHERE (sender_id = $1 AND recipient_id = $2)
               OR (sender_id = $2 AND recipient_id = $1)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            user_a,
            user_b,
            i64::from(limit as i32),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(records
            .into_iter()
            .map(|row| PrivateMessage {
                id: row.id,
                sender_id: row.sender_id,
                recipient_id: row.recipient_id,
                content: row.content,
                created_at: row.created_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pg::user::PgUserRepo;
    use crate::domain::user::{Email, User};
    use crate::ports::UserRepo;

    async fn seed_user(pool: &PgPool) -> Uuid {
        let users = PgUserRepo::new(pool.clone());
        let email = Email::new(format!("pm_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();
        user.id
    }

    #[sqlx::test]
    async fn it_persists_and_reads_a_conversation_both_directions(pool: PgPool) {
        let repo = PgPrivateMessageRepo::new(pool.clone());
        let alice = seed_user(&pool).await;
        let bob = seed_user(&pool).await;

        let m1 = PrivateMessage::new(alice, bob, "hey bob").unwrap();
        let m2 = PrivateMessage::new(bob, alice, "hey alice").unwrap();
        repo.save(&m1).await.unwrap();
        repo.save(&m2).await.unwrap();

        // Either participant sees both directions; argument order is irrelevant.
        let from_alice = repo.list_conversation(alice, bob, 50).await.unwrap();
        let from_bob = repo.list_conversation(bob, alice, 50).await.unwrap();
        assert_eq!(from_alice.len(), 2);
        assert_eq!(from_bob.len(), 2);
    }

    #[sqlx::test]
    async fn it_scopes_a_conversation_to_its_two_participants(pool: PgPool) {
        let repo = PgPrivateMessageRepo::new(pool.clone());
        let alice = seed_user(&pool).await;
        let bob = seed_user(&pool).await;
        let carol = seed_user(&pool).await;

        repo.save(&PrivateMessage::new(alice, bob, "private a->b").unwrap())
            .await
            .unwrap();
        repo.save(&PrivateMessage::new(alice, carol, "private a->c").unwrap())
            .await
            .unwrap();

        // The alice<->bob conversation never includes the alice<->carol message.
        let ab = repo.list_conversation(alice, bob, 50).await.unwrap();
        assert_eq!(ab.len(), 1);
        assert_eq!(ab[0].content, "private a->b");

        // Carol and bob never exchanged anything.
        let cb = repo.list_conversation(carol, bob, 50).await.unwrap();
        assert!(cb.is_empty());
    }

    #[sqlx::test]
    async fn it_returns_newest_first_and_respects_the_limit(pool: PgPool) {
        let repo = PgPrivateMessageRepo::new(pool.clone());
        let alice = seed_user(&pool).await;
        let bob = seed_user(&pool).await;

        // Three messages with strictly increasing timestamps.
        let mut first = PrivateMessage::new(alice, bob, "one").unwrap();
        let mut second = PrivateMessage::new(alice, bob, "two").unwrap();
        let third = PrivateMessage::new(bob, alice, "three").unwrap();
        first.created_at = third.created_at - chrono::Duration::seconds(2);
        second.created_at = third.created_at - chrono::Duration::seconds(1);
        repo.save(&first).await.unwrap();
        repo.save(&second).await.unwrap();
        repo.save(&third).await.unwrap();

        let latest_two = repo.list_conversation(alice, bob, 2).await.unwrap();
        assert_eq!(latest_two.len(), 2);
        assert_eq!(latest_two[0].content, "three");
        assert_eq!(latest_two[1].content, "two");
    }
}
