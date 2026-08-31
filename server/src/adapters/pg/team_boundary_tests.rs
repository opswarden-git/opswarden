use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::pg::team::PgTeamRepo;
use crate::adapters::pg::user::PgUserRepo;
use crate::domain::team::{Role, Team};
use crate::domain::user::{Email, User};
use crate::ports::{TeamRepo, UserRepo};

async fn user(pool: &PgPool) -> Uuid {
    let user = User::new(
        Email::new(format!("boundary-{}@opswarden.test", Uuid::new_v4())).unwrap(),
        "hash",
    );
    PgUserRepo::new(pool.clone()).save(&user).await.unwrap();
    user.id
}

async fn team(pool: &PgPool, manager: Uuid, name: &str) -> Uuid {
    let team = Team::new(name).unwrap();
    PgTeamRepo::new(pool.clone())
        .create_team_with_manager(&team, manager)
        .await
        .unwrap();
    team.id
}

async fn incident(pool: &PgPool, team_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into incidents (id, team_id, title, status, severity, created_at) \
         values ($1, $2, 'Boundary', 'open', 'low', now())",
    )
    .bind(id)
    .bind(team_id)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn release(pool: &PgPool, team_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into releases (id, team_id, title, base_state, created_at) \
         values ($1, $2, 'Boundary', 'created', now())",
    )
    .bind(id)
    .bind(team_id)
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test]
async fn incident_assignment_is_scoped_and_cleared_when_membership_ends(pool: PgPool) {
    let manager_a = user(&pool).await;
    let manager_b = user(&pool).await;
    let team_a = team(&pool, manager_a, "Assignment A").await;
    let team_b = team(&pool, manager_b, "Assignment B").await;
    let outsider = user(&pool).await;
    PgTeamRepo::new(pool.clone())
        .add_member(team_b, outsider, Role::Responder)
        .await
        .unwrap();

    let rejected = sqlx::query(
        "insert into incidents (id, team_id, title, status, severity, assignee_id, created_at) \
         values ($1, $2, 'Cross-team', 'open', 'high', $3, now())",
    )
    .bind(Uuid::new_v4())
    .bind(team_a)
    .bind(outsider)
    .execute(&pool)
    .await;
    assert!(rejected.is_err());

    PgTeamRepo::new(pool.clone())
        .add_member(team_a, outsider, Role::Responder)
        .await
        .unwrap();
    let incident_id = incident(&pool, team_a).await;
    sqlx::query("update incidents set assignee_id = $2 where id = $1")
        .bind(incident_id)
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    PgTeamRepo::new(pool.clone())
        .remove_member(team_a, outsider)
        .await
        .unwrap();

    let assignee: Option<Uuid> =
        sqlx::query_scalar("select assignee_id from incidents where id = $1")
            .bind(incident_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(assignee, None);
}

#[sqlx::test]
async fn timeline_authors_must_be_members_but_history_and_system_entries_survive(pool: PgPool) {
    let manager_a = user(&pool).await;
    let manager_b = user(&pool).await;
    let team_a = team(&pool, manager_a, "Timeline A").await;
    let team_b = team(&pool, manager_b, "Timeline B").await;
    let author = user(&pool).await;
    PgTeamRepo::new(pool.clone())
        .add_member(team_b, author, Role::Responder)
        .await
        .unwrap();
    let incident_id = incident(&pool, team_a).await;

    let rejected = sqlx::query(
        "insert into timeline_entries (id, incident_id, author_id, content, created_at) \
         values ($1, $2, $3, 'Cross-team', now())",
    )
    .bind(Uuid::new_v4())
    .bind(incident_id)
    .bind(author)
    .execute(&pool)
    .await;
    assert!(rejected.is_err());

    PgTeamRepo::new(pool.clone())
        .add_member(team_a, author, Role::Responder)
        .await
        .unwrap();
    let authored_id = Uuid::new_v4();
    sqlx::query(
        "insert into timeline_entries (id, incident_id, author_id, content, created_at) \
         values ($1, $2, $3, 'Valid', now())",
    )
    .bind(authored_id)
    .bind(incident_id)
    .bind(author)
    .execute(&pool)
    .await
    .unwrap();
    PgTeamRepo::new(pool.clone())
        .remove_member(team_a, author)
        .await
        .unwrap();
    let retained: Option<Uuid> =
        sqlx::query_scalar("select author_id from timeline_entries where id = $1")
            .bind(authored_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained, Some(author));

    sqlx::query(
        "insert into timeline_entries (id, incident_id, author_id, content, created_at) \
         values ($1, $2, null, 'System', now())",
    )
    .bind(Uuid::new_v4())
    .bind(incident_id)
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn release_validators_and_incident_links_cannot_cross_team_boundaries(pool: PgPool) {
    let manager_a = user(&pool).await;
    let manager_b = user(&pool).await;
    let team_a = team(&pool, manager_a, "Release A").await;
    let team_b = team(&pool, manager_b, "Release B").await;
    let release_id = release(&pool, team_a).await;
    let incident_a = incident(&pool, team_a).await;
    let incident_b = incident(&pool, team_b).await;
    let validator = user(&pool).await;
    PgTeamRepo::new(pool.clone())
        .add_member(team_a, validator, Role::Responder)
        .await
        .unwrap();
    sqlx::query("insert into release_steps (release_id, position, name) values ($1, 0, 'Deploy')")
        .bind(release_id)
        .execute(&pool)
        .await
        .unwrap();

    let validation = sqlx::query(
        "update release_steps set validated_by = $2, validated_at = now() \
         where release_id = $1 and position = 0",
    )
    .bind(release_id)
    .bind(manager_b)
    .execute(&pool)
    .await;
    assert!(validation.is_err());

    sqlx::query(
        "update release_steps set validated_by = $2, validated_at = now() \
         where release_id = $1 and position = 0",
    )
    .bind(release_id)
    .bind(validator)
    .execute(&pool)
    .await
    .unwrap();
    PgTeamRepo::new(pool.clone())
        .remove_member(team_a, validator)
        .await
        .unwrap();
    sqlx::query(
        "update release_steps set validated_by = $2, validated_at = now() \
         where release_id = $1 and position = 0",
    )
    .bind(release_id)
    .bind(validator)
    .execute(&pool)
    .await
    .unwrap();

    let mut transaction = pool.begin().await.unwrap();
    sqlx::query(
        "insert into release_incidents (team_id, release_id, incident_id) values ($1, $2, $3)",
    )
    .bind(team_a)
    .bind(release_id)
    .bind(incident_a)
    .execute(&mut *transaction)
    .await
    .unwrap();
    let cross_link = sqlx::query(
        "insert into release_incidents (team_id, release_id, incident_id) values ($1, $2, $3)",
    )
    .bind(team_a)
    .bind(release_id)
    .bind(incident_b)
    .execute(&mut *transaction)
    .await;
    assert!(cross_link.is_err());
    transaction.rollback().await.unwrap();
    let link_count: i64 =
        sqlx::query_scalar("select count(*) from release_incidents where release_id = $1")
            .bind(release_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(link_count, 0);

    sqlx::query(
        "insert into release_incidents (team_id, release_id, incident_id) values ($1, $2, $3)",
    )
    .bind(team_a)
    .bind(release_id)
    .bind(incident_a)
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn legacy_release_link_insert_derives_the_team(pool: PgPool) {
    let manager = user(&pool).await;
    let team_id = team(&pool, manager, "Legacy link").await;
    let release_id = release(&pool, team_id).await;
    let incident_id = incident(&pool, team_id).await;

    sqlx::query("insert into release_incidents (release_id, incident_id) values ($1, $2)")
        .bind(release_id)
        .bind(incident_id)
        .execute(&pool)
        .await
        .unwrap();

    let stored_team: Uuid = sqlx::query_scalar(
        "select team_id from release_incidents where release_id = $1 and incident_id = $2",
    )
    .bind(release_id)
    .bind(incident_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored_team, team_id);
}

#[sqlx::test]
async fn legacy_release_link_insert_still_rejects_a_cross_team_incident(pool: PgPool) {
    let manager_a = user(&pool).await;
    let manager_b = user(&pool).await;
    let team_a = team(&pool, manager_a, "Legacy link A").await;
    let team_b = team(&pool, manager_b, "Legacy link B").await;
    let release_id = release(&pool, team_a).await;
    let incident_id = incident(&pool, team_b).await;

    let result =
        sqlx::query("insert into release_incidents (release_id, incident_id) values ($1, $2)")
            .bind(release_id)
            .bind(incident_id)
            .execute(&pool)
            .await;

    assert!(result.is_err());
}

#[sqlx::test]
async fn active_bans_and_memberships_are_mutually_exclusive_even_when_concurrent(pool: PgPool) {
    let manager = user(&pool).await;
    let target = user(&pool).await;
    let team_id = team(&pool, manager, "Concurrent access").await;

    let add_member = sqlx::query(
        "insert into team_members (team_id, user_id, role) values ($1, $2, 'observer')",
    )
    .bind(team_id)
    .bind(target)
    .execute(&pool);
    let add_ban =
        sqlx::query("insert into team_bans (team_id, user_id, created_by) values ($1, $2, $3)")
            .bind(team_id)
            .bind(target)
            .bind(manager)
            .execute(&pool);
    let (member_result, ban_result) = tokio::join!(add_member, add_ban);
    assert_eq!(
        usize::from(member_result.is_ok()) + usize::from(ban_result.is_ok()),
        1
    );

    sqlx::query("delete from team_members where team_id = $1 and user_id = $2")
        .bind(team_id)
        .bind(target)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from team_bans where team_id = $1 and user_id = $2")
        .bind(team_id)
        .bind(target)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "insert into team_bans (team_id, user_id, expires_at, created_by) \
         values ($1, $2, $3, $4)",
    )
    .bind(team_id)
    .bind(target)
    .bind(Utc::now() - Duration::minutes(1))
    .bind(manager)
    .execute(&pool)
    .await
    .unwrap();
    PgTeamRepo::new(pool.clone())
        .add_member(team_id, target, Role::Observer)
        .await
        .unwrap();
}
