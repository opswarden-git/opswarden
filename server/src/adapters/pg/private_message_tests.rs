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
async fn hydrates_attachment_metadata_and_viewer_reactions(pool: PgPool) {
    let repo = PgPrivateMessageRepo::new(pool.clone());
    let alice = seed_user(&pool).await;
    let bob = seed_user(&pool).await;
    let message = PrivateMessage::new_with_attachments(
        alice,
        bob,
        "runbook",
        vec![(
            "runbook.pdf".into(),
            "application/pdf".into(),
            vec![1, 2, 3],
        )],
    )
    .unwrap();
    let attachment_id = message.attachments[0].id;
    repo.save(&message).await.unwrap();
    assert!(repo.toggle_reaction(message.id, alice, "✅").await.unwrap());

    let page = repo.list_conversation(alice, bob, None, 50).await.unwrap();
    assert_eq!(page[0].attachments[0].size_bytes, 3);
    assert!(page[0].attachments[0].content.is_empty());
    assert_eq!(page[0].reactions[0].count, 1);
    assert!(page[0].reactions[0].reacted);

    let download = repo
        .find_attachment_for_participant(attachment_id, bob)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(download.content, vec![1, 2, 3]);
    assert!(repo
        .find_attachment_for_participant(attachment_id, seed_user(&pool).await)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        repo.find_participants(message.id).await.unwrap(),
        Some((alice, bob))
    );
}

#[sqlx::test]
async fn conversation_reads_never_leak_a_third_participants_messages(pool: PgPool) {
    let repo = PgPrivateMessageRepo::new(pool.clone());
    let alice = seed_user(&pool).await;
    let bob = seed_user(&pool).await;
    let carol = seed_user(&pool).await;
    let alice_to_bob = PrivateMessage::new(alice, bob, "for bob").unwrap();
    let alice_to_carol = PrivateMessage::new(alice, carol, "for carol").unwrap();
    repo.save(&alice_to_bob).await.unwrap();
    repo.save(&alice_to_carol).await.unwrap();

    let conversation = repo.list_conversation(alice, bob, None, 50).await.unwrap();

    assert_eq!(conversation.len(), 1);
    assert_eq!(conversation[0].id, alice_to_bob.id);
}

#[sqlx::test]
async fn keyset_history_and_edits_are_stable(pool: PgPool) {
    let repo = PgPrivateMessageRepo::new(pool.clone());
    let alice = seed_user(&pool).await;
    let bob = seed_user(&pool).await;
    let mut first = PrivateMessage::new(alice, bob, "one").unwrap();
    let second = PrivateMessage::new(bob, alice, "two").unwrap();
    first.created_at = second.created_at - chrono::Duration::seconds(1);
    repo.save(&first).await.unwrap();
    repo.save(&second).await.unwrap();

    let latest = repo.list_conversation(alice, bob, None, 1).await.unwrap();
    assert_eq!(latest[0].id, second.id);
    let older = repo
        .list_conversation(alice, bob, Some((second.created_at, second.id)), 1)
        .await
        .unwrap();
    assert_eq!(older[0].id, first.id);

    let edited_at = Utc::now();
    repo.update_content(first.id, "edited", edited_at)
        .await
        .unwrap();
    let reloaded = repo.list_conversation(alice, bob, None, 10).await.unwrap();
    let edited = reloaded
        .iter()
        .find(|message| message.id == first.id)
        .unwrap();
    assert_eq!(edited.content, "edited");
    assert_eq!(
        edited.edited_at.map(|value| value.timestamp_micros()),
        Some(edited_at.timestamp_micros())
    );
}
