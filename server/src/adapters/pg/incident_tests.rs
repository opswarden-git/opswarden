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
    teams.save_team(&team).await.unwrap();
    teams
        .add_member(team.id, user.id, Role::Manager)
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
        .list_events_for_incident(incident.id, 10)
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
