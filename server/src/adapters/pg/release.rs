// --- server/src/adapters/pg/release.rs ---
//
// Postgres adapter for releases. The release row stores only the base lifecycle
// state; `blocked` is computed by callers from `count_active_linked_incidents`,
// which is the single SQL join over `incidents.status` that makes auto-unblock
// fall out of an incident resolving.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::release::{Release, ReleaseState, ReleaseStep};
use crate::ports::ReleaseRepo;

pub struct PgReleaseRepo {
    pool: PgPool,
}

impl PgReleaseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn base_state_to_str(state: ReleaseState) -> &'static str {
    match state {
        ReleaseState::Created => "created",
        ReleaseState::InProgress => "in_progress",
        ReleaseState::Completed => "completed",
        ReleaseState::Cancelled => "cancelled",
        // `Blocked` is a derived effective state, never a stored base; map it back
        // to its underlying base defensively so a leak can never corrupt the row.
        ReleaseState::Blocked => "in_progress",
    }
}

#[async_trait]
impl ReleaseRepo for PgReleaseRepo {
    async fn save_release(&self, release: &Release) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        sqlx::query!(
            r#"
            INSERT INTO releases (id, team_id, title, base_state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            release.id,
            release.team_id,
            release.title,
            base_state_to_str(release.base_state),
            release.created_at,
            release.updated_at,
        )
        .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }

        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
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
            base_state: ReleaseState::from_base_str(&row.base_state)?,
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
                    base_state: ReleaseState::from_base_str(&row.base_state)?,
                    steps: steps_by_release.remove(&row.id).unwrap_or_default(),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn update_release(&self, release: &Release) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|_| DomainError::Storage)?;

        sqlx::query!(
            r#"UPDATE releases SET base_state = $2, updated_at = $3 WHERE id = $1"#,
            release.id,
            base_state_to_str(release.base_state),
            release.updated_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| DomainError::Storage)?;

        for step in &release.steps {
            sqlx::query!(
                r#"
                UPDATE release_steps SET validated_by = $3, validated_at = $4
                WHERE release_id = $1 AND position = $2
                "#,
                release.id,
                step.position,
                step.validated_by,
                step.validated_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| DomainError::Storage)?;
        }

        tx.commit().await.map_err(|_| DomainError::Storage)?;
        Ok(())
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
    ) -> Result<Vec<(Uuid, Uuid, ReleaseState)>, DomainError> {
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
                    ReleaseState::from_base_str(&row.base_state)?,
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
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::team::{Role, Team};
    use crate::domain::user::{Email, User};
    use crate::ports::{IncidentRepo, TeamRepo, UserRepo};

    async fn seed_team(pool: &PgPool) -> (Uuid, Uuid) {
        let users = PgUserRepo::new(pool.clone());
        let teams = PgTeamRepo::new(pool.clone());
        let email = Email::new(format!("release_it_{}@opswarden.com", Uuid::new_v4())).unwrap();
        let user = User::new(email, "hash");
        users.save(&user).await.unwrap();
        let team = Team::new("Release Team").unwrap();
        teams.save_team(&team).await.unwrap();
        teams
            .add_member(team.id, user.id, Role::Manager)
            .await
            .unwrap();
        (team.id, user.id)
    }

    #[sqlx::test]
    async fn it_saves_loads_and_validates_a_release(pool: PgPool) {
        let repo = PgReleaseRepo::new(pool.clone());
        let (team_id, user_id) = seed_team(&pool).await;

        let mut release =
            Release::new(team_id, "v1.0.0", vec!["build".into(), "prod".into()]).unwrap();
        repo.save_release(&release).await.unwrap();

        let loaded = repo.find_release_by_id(release.id).await.unwrap().unwrap();
        assert_eq!(loaded.base_state, ReleaseState::Created);
        assert_eq!(loaded.steps.len(), 2);
        assert_eq!(loaded.steps[0].name, "build");

        release.validate_step("build", user_id, false).unwrap();
        repo.update_release(&release).await.unwrap();

        let reloaded = repo.find_release_by_id(release.id).await.unwrap().unwrap();
        assert_eq!(reloaded.base_state, ReleaseState::InProgress);
        assert!(reloaded.steps[0].is_validated());
        assert_eq!(reloaded.steps[0].validated_by, Some(user_id));
        assert!(!reloaded.steps[1].is_validated());

        let listed = repo.list_releases_for_team(team_id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].steps.len(), 2);
    }

    #[sqlx::test]
    async fn linked_active_incident_blocks_and_resolving_unblocks(pool: PgPool) {
        let releases = PgReleaseRepo::new(pool.clone());
        let incidents = PgIncidentRepo::new(pool.clone());
        let (team_id, _user) = seed_team(&pool).await;

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
        assert_eq!(linked, vec![(release.id, team_id, ReleaseState::Created)]);

        // Resolve the incident → the active count drops to zero (auto-unblock).
        incident.resolve().unwrap();
        incidents.update_incident(&incident).await.unwrap();
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
