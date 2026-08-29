use super::*;
use crate::adapters::pg::team::PgTeamRepo;
use crate::adapters::pg::user::PgUserRepo;
use crate::domain::team::{Role, Team};
use crate::domain::user::{Email, User};
use crate::ports::{TeamRepo, UserRepo};
async fn seed_team(pool: &PgPool) -> Uuid {
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let email = Email::new(format!("incident_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let user = User::new(email, "hash");
    users.save(&user).await.unwrap();

    let team = Team::new("Incident Team").unwrap();
    teams
        .create_team_with_manager(&team, user.id)
        .await
        .unwrap();
    team.id
}

#[sqlx::test]
async fn it_saves_finds_and_updates_incidents_in_postgres(pool: PgPool) {
    let repo = PgIncidentRepo::new(pool.clone());
    let team_id = seed_team(&pool).await;

    let mut incident = Incident::new(team_id, "API saturation", Severity::High).unwrap();
    repo.save_incident(&incident).await.unwrap();

    let found = repo
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.title, "API saturation");
    assert_eq!(found.status, IncidentStatus::Open);

    incident.acknowledge().unwrap();
    repo.update_incident(&incident).await.unwrap();

    let updated = repo
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, IncidentStatus::Acknowledged);
}

#[sqlx::test]
async fn clear_assignee_for_member_unassigns_their_incidents(pool: PgPool) {
    let repo = PgIncidentRepo::new(pool.clone());
    let team_id = seed_team(&pool).await;

    // A user to assign, then "remove" from the team.
    let users = PgUserRepo::new(pool.clone());
    let email = Email::new(format!("assignee_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let assignee = User::new(email, "hash");
    users.save(&assignee).await.unwrap();

    let mut incident = Incident::new(team_id, "owned by a member", Severity::High).unwrap();
    incident.assign(assignee.id);
    repo.save_incident(&incident).await.unwrap();
    assert_eq!(
        repo.find_incident_by_id(incident.id)
            .await
            .unwrap()
            .unwrap()
            .assignee,
        Some(assignee.id)
    );

    repo.clear_assignee_for_member(team_id, assignee.id)
        .await
        .unwrap();

    assert_eq!(
        repo.find_incident_by_id(incident.id)
            .await
            .unwrap()
            .unwrap()
            .assignee,
        None
    );
}

#[sqlx::test]
async fn incident_and_initial_event_are_committed_together(pool: PgPool) {
    let repo = PgIncidentRepo::new(pool.clone());
    let team_id = seed_team(&pool).await;
    let incident = Incident::new(team_id, "API saturation", Severity::Critical).unwrap();
    let event = IncidentEvent::created(&incident, None);

    repo.save_incident_with_event(&incident, &event)
        .await
        .unwrap();

    let events = repo
        .list_events_for_incident(incident.id, None, 10)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, IncidentEventKind::Created);
}

#[sqlx::test]
async fn a_failed_event_rolls_back_the_incident_write(pool: PgPool) {
    let repo = PgIncidentRepo::new(pool.clone());
    let team_id = seed_team(&pool).await;
    let incident = Incident::new(team_id, "Must roll back", Severity::High).unwrap();
    let mut invalid_event = IncidentEvent::created(&incident, None);
    invalid_event.incident_id = Uuid::new_v4();

    assert_eq!(
        repo.save_incident_with_event(&incident, &invalid_event)
            .await
            .unwrap_err(),
        DomainError::Storage
    );
    assert!(repo
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test]
async fn a_failed_event_rolls_back_the_incident_update(pool: PgPool) {
    let repo = PgIncidentRepo::new(pool.clone());
    let team_id = seed_team(&pool).await;
    let mut incident = Incident::new(team_id, "Stable state", Severity::High).unwrap();
    repo.save_incident(&incident).await.unwrap();
    incident.acknowledge().unwrap();
    let invalid_event = IncidentEvent::status_changed(
        Uuid::new_v4(),
        Uuid::new_v4(),
        IncidentStatus::Open,
        IncidentStatus::Acknowledged,
    );

    assert_eq!(
        repo.update_incident_with_event(&incident, &invalid_event)
            .await
            .unwrap_err(),
        DomainError::Storage
    );
    assert_eq!(
        repo.find_incident_by_id(incident.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        IncidentStatus::Open
    );
}

#[sqlx::test]
async fn incident_read_position_clears_unread_and_never_moves_backwards(pool: PgPool) {
    let incidents = PgIncidentRepo::new(pool.clone());
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let viewer_email = Email::new(format!("viewer_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let viewer = User::new(viewer_email, "hash");
    users.save(&viewer).await.unwrap();
    let actor_email = Email::new(format!("actor_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let actor = User::new(actor_email, "hash");
    users.save(&actor).await.unwrap();
    let team = Team::new("Unread Team").unwrap();
    teams
        .create_team_with_manager(&team, actor.id)
        .await
        .unwrap();
    teams
        .add_member(team.id, viewer.id, Role::Observer)
        .await
        .unwrap();

    let incident = Incident::new(team.id, "Unread incident", Severity::High).unwrap();
    let event = IncidentEvent::created(&incident, Some(actor.id));
    incidents
        .save_incident_with_event(&incident, &event)
        .await
        .unwrap();
    assert_eq!(
        incidents
            .list_unread_incident_ids(team.id, viewer.id)
            .await
            .unwrap(),
        vec![incident.id]
    );

    let read_through = Utc::now();
    incidents
        .mark_incident_read(incident.id, viewer.id, read_through)
        .await
        .unwrap();
    incidents
        .mark_incident_read(
            incident.id,
            viewer.id,
            read_through - chrono::Duration::hours(1),
        )
        .await
        .unwrap();
    let stored: DateTime<Utc> = sqlx::query_scalar(
        "SELECT read_through FROM incident_channel_reads WHERE incident_id = $1 AND user_id = $2",
    )
    .bind(incident.id)
    .bind(viewer.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    // PostgreSQL stores microseconds while chrono can carry nanoseconds.
    assert_eq!(stored.timestamp_micros(), read_through.timestamp_micros());
    assert!(incidents
        .list_unread_incident_ids(team.id, viewer.id)
        .await
        .unwrap()
        .is_empty());
}
