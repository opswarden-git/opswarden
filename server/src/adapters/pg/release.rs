// --- server/src/adapters/pg/release.rs ---
//
// Postgres adapter for releases. The release row stores only the base lifecycle
// state; `blocked` is computed by callers from `count_active_linked_incidents`,
// which is the single SQL join over `incidents.status` that makes auto-unblock
// fall out of an incident resolving.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::automation_catalog::OPSWARDEN_SERVICE;
use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::release::{Release, ReleaseBaseState, ReleaseStep};
use crate::ports::ReleaseRepo;

use super::incident::{insert_event, insert_incident};

pub struct PgReleaseRepo {
    pool: PgPool,
}

impl PgReleaseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert_release(
        transaction: &mut Transaction<'_, Postgres>,
        release: &Release,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO releases (id, team_id, title, base_state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            release.id,
            release.team_id,
            release.title,
            release.base_state.as_str(),
            release.created_at,
            release.updated_at,
        )
        .execute(&mut **transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        for step in &release.steps {
            sqlx::query!(
                r#"
                INSERT INTO release_steps (release_id, position, name, validated_by, validated_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                release.id,
                step.position,
                step.name,
                step.validated_by,
                step.validated_at,
            )
            .execute(&mut **transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        Ok(())
    }

    async fn update_release_rows(
        transaction: &mut Transaction<'_, Postgres>,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let updated = sqlx::query(
            r#"
            UPDATE releases SET base_state = $2, updated_at = $3
            WHERE id = $1 AND updated_at = $4
            "#,
        )
        .bind(release.id)
        .bind(release.base_state.as_str())
        .bind(release.updated_at)
        .bind(expected_updated_at)
        .execute(&mut **transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if updated.rows_affected() != 1 {
            return Err(DomainError::ConcurrentModification);
        }

        for step in &release.steps {
            let updated = sqlx::query(
                r#"
                UPDATE release_steps SET validated_by = $3, validated_at = $4
                WHERE release_id = $1 AND position = $2
                "#,
            )
            .bind(release.id)
            .bind(step.position)
            .bind(step.validated_by)
            .bind(step.validated_at)
            .execute(&mut **transaction)
            .await
            .map_err(|_| DomainError::Storage)?;
            if updated.rows_affected() != 1 {
                return Err(DomainError::ConcurrentModification);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ReleaseRepo for PgReleaseRepo {
    async fn save_release(&self, release: &Release) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::insert_release(&mut tx, release).await?;
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn create_release(
        &self,
        release: &Release,
        delivery_id: &str,
        event: &crate::domain::automation::ExternalEvent,
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::insert_release(&mut transaction, release).await?;
        let body = serde_json::to_vec(event).map_err(|_| DomainError::Storage)?;
        let result = sqlx::query(
            r#"
            INSERT INTO webhook_jobs (
                id, connection_id, expected_service, provider_delivery_id,
                provider_event, body
            )
            SELECT $1, connection.id, $2, $3, $4, $5
            FROM service_connections AS connection
            WHERE connection.team_id = $6 AND connection.service = $2
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(OPSWARDEN_SERVICE)
        .bind(delivery_id)
        .bind(&event.kind)
        .bind(body)
        .bind(release.team_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(DomainError::Storage);
        }
        transaction
            .commit()
            .await
            .map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn create_blocking_incident(
        &self,
        release_id: Uuid,
        expected_updated_at: DateTime<Utc>,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let locked = sqlx::query(
            r#"
            UPDATE releases
            SET updated_at = GREATEST(clock_timestamp(), updated_at + interval '1 microsecond')
            WHERE id = $1
              AND team_id = $2
              AND base_state = 'in_progress'
              AND updated_at = $3
            "#,
        )
        .bind(release_id)
        .bind(incident.team_id)
        .bind(expected_updated_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;
        if locked.rows_affected() != 1 {
            return Err(DomainError::ConcurrentModification);
        }

        insert_incident(&mut transaction, incident).await?;
        insert_event(&mut transaction, event).await?;
        sqlx::query(
            r#"
            INSERT INTO release_incidents (release_id, incident_id)
            VALUES ($1, $2)
            "#,
        )
        .bind(release_id)
        .bind(incident.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| DomainError::Storage)?;

        transaction.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn find_release_by_id(&self, release_id: Uuid) -> Result<Option<Release>, DomainError> {
        let row = sqlx::query!(
            r#"SELECT id, team_id, title, base_state, created_at, updated_at FROM releases WHERE id = $1"#,
            release_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let steps = sqlx::query!(
            r#"
            SELECT position, name, validated_by, validated_at
            FROM release_steps
            WHERE release_id = $1
            ORDER BY position
            "#,
            release_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        Ok(Some(Release {
            id: row.id,
            team_id: row.team_id,
            title: row.title,
            base_state: ReleaseBaseState::try_from(row.base_state.as_str())?,
            steps: steps
                .into_iter()
                .map(|s| ReleaseStep {
                    position: s.position,
                    name: s.name,
                    validated_by: s.validated_by,
                    validated_at: s.validated_at,
                })
                .collect(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, team_id, title, base_state, created_at, updated_at
            FROM releases
            WHERE team_id = $1
            ORDER BY created_at DESC
            "#,
            team_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let step_rows = sqlx::query!(
            r#"
            SELECT release_id, position, name, validated_by, validated_at
            FROM release_steps
            WHERE release_id = ANY($1)
            ORDER BY release_id, position
            "#,
            &ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        let mut steps_by_release: HashMap<Uuid, Vec<ReleaseStep>> = HashMap::new();
        for s in step_rows {
            steps_by_release
                .entry(s.release_id)
                .or_default()
                .push(ReleaseStep {
                    position: s.position,
                    name: s.name,
                    validated_by: s.validated_by,
                    validated_at: s.validated_at,
                });
        }

        rows.into_iter()
            .map(|row| {
                Ok(Release {
                    id: row.id,
                    team_id: row.team_id,
                    title: row.title,
                    base_state: ReleaseBaseState::try_from(row.base_state.as_str())?,
                    steps: steps_by_release.remove(&row.id).unwrap_or_default(),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn update_release(
        &self,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::update_release_rows(&mut tx, release, expected_updated_at).await?;
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn update_release_with_incident_events(
        &self,
        release: &Release,
        expected_updated_at: DateTime<Utc>,
        events: &[IncidentEvent],
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        Self::update_release_rows(&mut transaction, release, expected_updated_at).await?;
        for event in events {
            insert_event(&mut transaction, event).await?;
        }
        transaction.commit().await.map_err(|_| DomainError::Storage)
    }

    async fn link_incident(&self, release_id: Uuid, incident_id: Uuid) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let linked = sqlx::query!(
            r#"
            INSERT INTO release_incidents (release_id, incident_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            release_id,
            incident_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if linked.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE releases SET updated_at = now() WHERE id = $1"#,
                release_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn unlink_incident(
        &self,
        release_id: Uuid,
        incident_id: Uuid,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;
        let unlinked = sqlx::query!(
            r#"DELETE FROM release_incidents WHERE release_id = $1 AND incident_id = $2"#,
            release_id,
            incident_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;
        if unlinked.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE releases SET updated_at = now() WHERE id = $1"#,
                release_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }
        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
    }

    async fn list_linked_incident_ids(&self, release_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query!(
            r#"SELECT incident_id FROM release_incidents WHERE release_id = $1"#,
            release_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(rows.into_iter().map(|r| r.incident_id).collect())
    }

    async fn count_active_linked_incidents(&self, release_id: Uuid) -> Result<u64, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM release_incidents ri
            JOIN incidents i ON i.id = ri.incident_id
            WHERE ri.release_id = $1 AND i.status <> 'resolved'
            "#,
            release_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;
        Ok(row.count as u64)
    }

    async fn list_release_states_linked_to_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState)>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT r.id, r.team_id, r.base_state
            FROM release_incidents ri
            JOIN releases r ON r.id = ri.release_id
            WHERE ri.incident_id = $1
            "#,
            incident_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| DomainError::Storage)?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    row.id,
                    row.team_id,
                    ReleaseBaseState::try_from(row.base_state.as_str())?,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
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
        incident.acknowledge().unwrap(); // active (not resolved)
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
        // idempotent re-link
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

        // Resolve the incident → the active count drops to zero (auto-unblock).
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

        // Unlink is idempotent and removes the link.
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
}
