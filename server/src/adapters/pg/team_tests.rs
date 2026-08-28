use super::*;
use crate::adapters::pg::user::PgUserRepo;
use crate::domain::team::{BanKind, TeamImage};
use crate::domain::user::{Email, User};
use crate::ports::UserRepo;
use chrono::Utc;
/// Persist a throwaway user so membership FKs resolve.
async fn seed_user(pool: &PgPool) -> Uuid {
    let users = PgUserRepo::new(pool.clone());
    let email = Email::new(format!("team_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let user = User::new(email, "hash");
    users.save(&user).await.unwrap();
    user.id
}

#[sqlx::test]
async fn it_creates_joins_and_transfers_in_postgres(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());

    let manager = seed_user(&pool).await;
    let newcomer = seed_user(&pool).await;

    let team = Team::new("Postgres Crew").unwrap();
    repo.save_team(&team).await.unwrap();

    // Resolve by invitation code (the join entry point).
    let found = repo
        .find_by_invitation_code(team.invitation_code.as_str())
        .await
        .unwrap();
    assert_eq!(found.unwrap().id, team.id);

    // Creator is Manager, newcomer joins as Observer.
    repo.add_member(team.id, manager, Role::Manager)
        .await
        .unwrap();
    repo.add_member(team.id, newcomer, Role::Observer)
        .await
        .unwrap();
    assert_eq!(
        repo.find_member_role(team.id, manager).await.unwrap(),
        Some(Role::Manager)
    );

    // Atomic hand-over upholds the single-Manager invariant.
    repo.transfer_manager(team.id, manager, newcomer)
        .await
        .unwrap();
    assert_eq!(
        repo.find_member_role(team.id, manager).await.unwrap(),
        Some(Role::Responder)
    );
    assert_eq!(
        repo.find_member_role(team.id, newcomer).await.unwrap(),
        Some(Role::Manager)
    );
}

#[sqlx::test]
async fn joining_twice_is_rejected_by_the_database(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());

    let user = seed_user(&pool).await;
    let team = Team::new("Dup Guard").unwrap();
    repo.save_team(&team).await.unwrap();

    repo.add_member(team.id, user, Role::Observer)
        .await
        .unwrap();
    let again = repo.add_member(team.id, user, Role::Observer).await;

    assert_eq!(again.unwrap_err(), DomainError::AlreadyMember);
}

#[sqlx::test]
async fn unknown_invitation_code_returns_none(pool: PgPool) {
    let repo = PgTeamRepo::new(pool);

    let found = repo.find_by_invitation_code("OPS-NOPE99").await.unwrap();

    assert!(found.is_none());
}

#[sqlx::test]
async fn it_lists_members_with_email_and_role(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());

    let manager = seed_user(&pool).await;
    let observer = seed_user(&pool).await;
    let team = Team::new("Roster Crew").unwrap();
    repo.save_team(&team).await.unwrap();
    repo.add_member(team.id, manager, Role::Manager)
        .await
        .unwrap();
    repo.add_member(team.id, observer, Role::Observer)
        .await
        .unwrap();

    let members = repo.list_members(team.id).await.unwrap();

    assert_eq!(members.len(), 2);
    let manager_row = members.iter().find(|m| m.user_id == manager).unwrap();
    assert_eq!(manager_row.role, Role::Manager);
    assert!(manager_row.email.contains('@'));
    assert!(members
        .iter()
        .any(|m| m.user_id == observer && m.role == Role::Observer));
}

#[sqlx::test]
async fn it_lists_no_members_for_an_unknown_team(pool: PgPool) {
    let repo = PgTeamRepo::new(pool);

    let members = repo.list_members(Uuid::new_v4()).await.unwrap();

    assert!(members.is_empty());
}

#[sqlx::test]
async fn it_sets_a_member_role_in_postgres(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());

    let member = seed_user(&pool).await;
    let team = Team::new("Role Crew").unwrap();
    repo.save_team(&team).await.unwrap();
    repo.add_member(team.id, member, Role::Observer)
        .await
        .unwrap();

    repo.set_member_role(team.id, member, Role::Responder)
        .await
        .unwrap();

    assert_eq!(
        repo.find_member_role(team.id, member).await.unwrap(),
        Some(Role::Responder)
    );
}

#[sqlx::test]
async fn it_stores_finds_and_upserts_bans_in_postgres(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());
    let manager = seed_user(&pool).await;
    let target = seed_user(&pool).await;
    let team = Team::new("Ban Crew").unwrap();
    repo.save_team(&team).await.unwrap();

    // No ban initially.
    assert!(repo.find_ban(team.id, target).await.unwrap().is_none());

    // Permanent ban with a reason.
    let ban = TeamBan::permanent(team.id, target, manager, Some("spam".to_string()));
    repo.add_ban(&ban).await.unwrap();

    let found = repo.find_ban(team.id, target).await.unwrap().unwrap();
    assert!(matches!(found.kind, BanKind::Permanent));
    assert!(found.is_active(Utc::now()));
    assert_eq!(found.reason.as_deref(), Some("spam"));
    assert_eq!(found.created_by, Some(manager));

    // Re-banning the same user upserts (one row, now temporary).
    let expires = Utc::now() + chrono::Duration::hours(1);
    let temp = TeamBan::temporary(team.id, target, manager, expires, None).unwrap();
    repo.add_ban(&temp).await.unwrap();

    let bans = repo.list_bans(team.id).await.unwrap();
    assert_eq!(bans.len(), 1);
    assert!(matches!(bans[0].ban.kind, BanKind::Temporary { .. }));
    assert!(bans[0].ban.is_active(Utc::now()));
    assert!(bans[0].ban.reason.is_none());
}

#[sqlx::test]
async fn deleting_the_moderator_account_keeps_the_ban_and_nulls_created_by(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());
    let users = PgUserRepo::new(pool.clone());
    let moderator = seed_user(&pool).await;
    let target = seed_user(&pool).await;
    let team = Team::new("Ban Crew").unwrap();
    repo.save_team(&team).await.unwrap();

    repo.add_ban(&TeamBan::permanent(team.id, target, moderator, None))
        .await
        .unwrap();

    // The moderator deletes their account: the FK is ON DELETE SET NULL, so
    // this must not fail and the ban must survive.
    users.delete_account(moderator).await.unwrap();

    let ban = repo.find_ban(team.id, target).await.unwrap().unwrap();
    assert!(ban.is_active(Utc::now()));
    assert_eq!(ban.created_by, None);
}

#[sqlx::test]
async fn team_image_is_upserted_member_scoped_and_deleted(pool: PgPool) {
    let repo = PgTeamRepo::new(pool.clone());
    let member = seed_user(&pool).await;
    let outsider = seed_user(&pool).await;
    let team = Team::new("Image Crew").unwrap();
    repo.save_team(&team).await.unwrap();
    repo.add_member(team.id, member, Role::Manager)
        .await
        .unwrap();

    let first = TeamImage::new("image/png", b"\x89PNG\r\n\x1a\nfirst".to_vec()).unwrap();
    repo.save_team_image(team.id, &first).await.unwrap();
    assert_eq!(
        repo.find_team_image_for_member(team.id, member)
            .await
            .unwrap()
            .unwrap()
            .content,
        first.content
    );
    assert!(repo
        .find_team_image_for_member(team.id, outsider)
        .await
        .unwrap()
        .is_none());

    let second = TeamImage::new("image/jpeg", vec![0xff, 0xd8, 0xff, 0x01]).unwrap();
    repo.save_team_image(team.id, &second).await.unwrap();
    assert_eq!(
        repo.find_team_image_for_member(team.id, member)
            .await
            .unwrap()
            .unwrap()
            .content,
        second.content
    );

    repo.delete_team_image(team.id).await.unwrap();
    assert!(repo
        .find_team_image_for_member(team.id, member)
        .await
        .unwrap()
        .is_none());
}
