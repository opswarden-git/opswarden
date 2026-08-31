use super::*;
use crate::adapters::pg::incident::PgIncidentRepo;
use crate::adapters::pg::team::PgTeamRepo;
use crate::adapters::pg::user::PgUserRepo;
use crate::domain::automation::release_created_event;
use crate::domain::incident::{Incident, Severity};
use crate::domain::incident_event::IncidentEvent;
use crate::domain::team::Team;
use crate::domain::user::{Email, User};
use crate::ports::{IncidentRepo, TeamRepo, UserRepo};

async fn seed_team(pool: &PgPool) -> (Uuid, Uuid) {
    let users = PgUserRepo::new(pool.clone());
    let teams = PgTeamRepo::new(pool.clone());
    let email = Email::new(format!("release_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
    let user = User::new(email, "hash");
    users.save(&user).await.unwrap();
    let team = Team::new("Release Team").unwrap();
    teams
        .create_team_with_manager(&team, user.id)
        .await
        .unwrap();
    (team.id, user.id)
}

#[sqlx::test]
async fn release_and_internal_event_roll_back_together(pool: PgPool) {
    let (team_id, _) = seed_team(&pool).await;
    sqlx::query("DELETE FROM service_connections WHERE team_id = $1 AND service = 'opswarden'")
        .bind(team_id)
        .execute(&pool)
        .await
        .unwrap();
    let repo = PgReleaseRepo::new(pool.clone());
    let release = Release::new(team_id, "v3.0.0", vec!["deploy".into()]).unwrap();
    let event = release_created_event(&release);
    let delivery_id = format!("release:{}:created", release.id);

    assert_eq!(
        repo.create_release(&release, &delivery_id, &event).await,
        Err(DomainError::Storage)
    );
    assert!(repo.find_release_by_id(release.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn it_saves_loads_and_validates_a_release(pool: PgPool) {
    let repo = PgReleaseRepo::new(pool.clone());
    let (team_id, user_id) = seed_team(&pool).await;

    let mut release =
        Release::new(team_id, "v1.0.0", vec!["build".into(), "prod".into()]).unwrap();
    repo.save_release(&release).await.unwrap();

    let loaded = repo.find_release_by_id(release.id).await.unwrap().unwrap();
    assert_eq!(loaded.base_state, ReleaseBaseState::Created);
    assert_eq!(loaded.steps.len(), 2);
    assert_eq!(loaded.steps[0].name, "build");

    release.validate_step("build", user_id, false).unwrap();
    repo.update_release(&release, loaded.updated_at)
        .await
        .unwrap();

    let reloaded = repo.find_release_by_id(release.id).await.unwrap().unwrap();
    assert_eq!(reloaded.base_state, ReleaseBaseState::InProgress);
    assert!(reloaded.steps[0].is_validated());
    assert_eq!(reloaded.steps[0].validated_by, Some(user_id));
    assert!(!reloaded.steps[1].is_validated());

    let listed = repo.list_releases_for_team(team_id).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].steps.len(), 2);
}

#[sqlx::test]
async fn concurrent_cancel_and_validation_have_one_coherent_winner(pool: PgPool) {
    let repo = PgReleaseRepo::new(pool.clone());
    let (team_id, user_id) = seed_team(&pool).await;
    let release = Release::new(team_id, "v-race", vec!["build".into()]).unwrap();
    repo.save_release(&release).await.unwrap();

    let expected_updated_at = release.updated_at;
    let mut cancelled = release.clone();
    let mut validated = release.clone();
    cancelled.cancel().unwrap();
    validated.validate_step("build", user_id, false).unwrap();
    cancelled.updated_at = expected_updated_at + chrono::Duration::seconds(1);
    validated.updated_at = expected_updated_at + chrono::Duration::seconds(2);

    let (cancel_result, validate_result) = tokio::join!(
        repo.update_release(&cancelled, expected_updated_at),
        repo.update_release(&validated, expected_updated_at),
    );

    let results = [cancel_result, validate_result];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(DomainError::ConcurrentModification))
            .count(),
        1
    );
    let stored = repo.find_release_by_id(release.id).await.unwrap().unwrap();
    match stored.base_state {
        ReleaseBaseState::Cancelled => assert!(!stored.steps[0].is_validated()),
        ReleaseBaseState::Completed => assert!(stored.steps[0].is_validated()),
        state => panic!("unexpected winning state: {state:?}"),
    }
}

#[sqlx::test]
async fn release_validation_rolls_back_when_incident_history_fails(pool: PgPool) {
    let repo = PgReleaseRepo::new(pool.clone());
    let (team_id, user_id) = seed_team(&pool).await;
    let mut release = Release::new(team_id, "v-history", vec!["build".into()]).unwrap();
    repo.save_release(&release).await.unwrap();
    let expected_updated_at = release.updated_at;
    release.validate_step("build", user_id, false).unwrap();
    let invalid_event = IncidentEvent::release_step_validated(
        Uuid::new_v4(),
        user_id,
        release.id,
        &release.title,
        "build",
    );

    assert_eq!(
        repo.update_release_with_incident_events(
            &release,
            expected_updated_at,
            &[invalid_event],
        )
        .await,
        Err(DomainError::Storage)
    );

    let stored = repo.find_release_by_id(release.id).await.unwrap().unwrap();
    assert_eq!(stored.base_state, ReleaseBaseState::Created);
    assert!(!stored.steps[0].is_validated());
}

#[sqlx::test]
async fn blocking_incident_and_release_link_roll_back_together(pool: PgPool) {
    let releases = PgReleaseRepo::new(pool.clone());
    let incidents = PgIncidentRepo::new(pool.clone());
    let (team_id, user_id) = seed_team(&pool).await;
    let mut release =
        Release::new(team_id, "v-block", vec!["build".into(), "deploy".into()]).unwrap();
    release.validate_step("build", user_id, false).unwrap();
    releases.save_release(&release).await.unwrap();
    let stored_updated_at = releases
        .find_release_by_id(release.id)
        .await
        .unwrap()
        .unwrap()
        .updated_at;
    let incident = Incident::new(team_id, "Deployment blocked", Severity::High).unwrap();
    let event = IncidentEvent::created(&incident, None);

    sqlx::query(
        r#"
        CREATE FUNCTION reject_release_incident_link() RETURNS trigger AS $$
        BEGIN
            RAISE EXCEPTION 'injected release link failure';
        END;
        $$ LANGUAGE plpgsql
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        CREATE TRIGGER reject_release_incident_link
        BEFORE INSERT ON release_incidents
        FOR EACH ROW EXECUTE FUNCTION reject_release_incident_link()
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        releases
            .create_blocking_incident(release.id, release.updated_at, &incident, &event)
            .await,
        Err(DomainError::Storage)
    );

    assert!(incidents
        .find_incident_by_id(incident.id)
        .await
        .unwrap()
        .is_none());
    assert!(releases
        .list_linked_incident_ids(release.id)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        releases
            .find_release_by_id(release.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at,
        stored_updated_at
    );
}

#[sqlx::test]
async fn linked_active_incident_blocks_and_resolving_unblocks(pool: PgPool) {
    let releases = PgReleaseRepo::new(pool.clone());
    let incidents = PgIncidentRepo::new(pool.clone());
    let (team_id, user) = seed_team(&pool).await;

    let release = Release::new(team_id, "v2.0.0", vec!["build".into()]).unwrap();
    releases.save_release(&release).await.unwrap();

    let mut incident = Incident::new(team_id, "DB down", Severity::Critical).unwrap();
    incident.acknowledge().unwrap();
    incidents.save_incident(&incident).await.unwrap();

    releases
        .link_incident(release.id, incident.id)
        .await
        .unwrap();
    let linked_at = releases
        .find_release_by_id(release.id)
        .await
        .unwrap()
        .unwrap()
        .updated_at;
    assert!(linked_at > release.updated_at);

    releases
        .link_incident(release.id, incident.id)
        .await
        .unwrap();
    assert_eq!(
        releases
            .find_release_by_id(release.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at,
        linked_at
    );

    assert_eq!(
        releases
            .count_active_linked_incidents(release.id)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        releases
            .list_linked_incident_ids(release.id)
            .await
            .unwrap()
            .len(),
        1
    );
    let linked = releases
        .list_release_states_linked_to_incident(incident.id)
        .await
        .unwrap();
    assert_eq!(
        linked,
        vec![(release.id, team_id, ReleaseBaseState::Created)]
    );

    let expected_updated_at = incident.updated_at;
    let previous_status = incident.status;
    incident.resolve().unwrap();
    let event =
        IncidentEvent::status_changed(incident.id, user, previous_status, incident.status);
    incidents
        .update_incident_with_event(&incident, &event, expected_updated_at)
        .await
        .unwrap();
    assert_eq!(
        releases
            .count_active_linked_incidents(release.id)
            .await
            .unwrap(),
        0
    );

    releases
        .unlink_incident(release.id, incident.id)
        .await
        .unwrap();
    let unlinked_at = releases
        .find_release_by_id(release.id)
        .await
        .unwrap()
        .unwrap()
        .updated_at;
    assert!(unlinked_at > linked_at);
    releases
        .unlink_incident(release.id, incident.id)
        .await
        .unwrap();
    assert_eq!(
        releases
            .find_release_by_id(release.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at,
        unlinked_at
    );
    assert!(releases
        .list_linked_incident_ids(release.id)
        .await
        .unwrap()
        .is_empty());
}
