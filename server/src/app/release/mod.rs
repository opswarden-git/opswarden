// --- server/src/app/release/mod.rs ---
//
// Release use-cases. Authorization: Observer read-only,
// Responder+ links and validates, Manager-only creates and cancels. The `blocked`
// effective state is derived (never stored) from active linked incidents, so the
// shared helpers here are the single place that turns a base-state change into a
// `release_state_changed` event — including the auto-unblock triggered from
// `ChangeIncidentStatus` when the last active linked incident resolves.

use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::event::DomainEvent;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::release::{effective_release_state, Release, ReleaseBaseState, ReleaseState};
use crate::ports::{EventPublisher, ReleaseRepo};

pub mod cancel_release;
pub mod create_release;
pub mod get_release;
pub mod link_incident;
pub mod list_releases;
pub mod validate_release_step;

pub use cancel_release::{CancelReleaseCommand, CancelReleaseUseCase};
pub use create_release::{CreateReleaseCommand, CreateReleaseUseCase};
pub use get_release::{GetReleaseCommand, GetReleaseUseCase};
pub use link_incident::{
    LinkIncidentCommand, LinkIncidentUseCase, UnlinkIncidentCommand, UnlinkIncidentUseCase,
};
pub use list_releases::{ListReleasesCommand, ListReleasesUseCase, ReleaseListItem};
pub use validate_release_step::{ValidateReleaseStepCommand, ValidateReleaseStepUseCase};

/// A release plus the read-only facts derived from its links: the effective
/// state (with `blocked` resolved) and the incidents currently linked to it.
#[derive(Debug, PartialEq, Eq)]
pub struct ReleaseDetail {
    pub release: Release,
    pub effective_state: ReleaseState,
    pub linked_incident_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseIncident {
    pub incident_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub severity: Severity,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReleaseIncidentProjection {
    pub linked: Vec<ReleaseIncident>,
    pub blockers: Vec<ReleaseIncident>,
    pub linkable: Vec<ReleaseIncident>,
}

pub fn project_release_incidents(
    linked_incident_ids: &[Uuid],
    incidents_by_id: &HashMap<Uuid, Incident>,
) -> ReleaseIncidentProjection {
    let linked: Vec<_> = linked_incident_ids
        .iter()
        .filter_map(|id| incidents_by_id.get(id))
        .map(release_incident)
        .collect();
    let blockers = linked
        .iter()
        .filter(|incident| incident.status != IncidentStatus::Resolved)
        .cloned()
        .collect();
    let mut linkable: Vec<_> = incidents_by_id
        .values()
        .filter(|incident| {
            incident.status != IncidentStatus::Resolved
                && !linked_incident_ids.contains(&incident.id)
        })
        .map(release_incident)
        .collect();
    linkable.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then(left.incident_id.cmp(&right.incident_id))
    });
    ReleaseIncidentProjection {
        linked,
        blockers,
        linkable,
    }
}

fn release_incident(incident: &Incident) -> ReleaseIncident {
    ReleaseIncident {
        incident_id: incident.id,
        title: incident.title.clone(),
        status: incident.status,
        severity: incident.severity,
    }
}

/// Enrich a release with its effective state and linked incidents for a read.
pub(crate) async fn load_detail(
    releases: &Arc<dyn ReleaseRepo>,
    release: Release,
) -> Result<ReleaseDetail, DomainError> {
    let has_active = releases.count_active_linked_incidents(release.id).await? > 0;
    let linked_incident_ids = releases.list_linked_incident_ids(release.id).await?;
    let effective_state = release.effective_state(has_active);
    Ok(ReleaseDetail {
        release,
        effective_state,
        linked_incident_ids,
    })
}

/// Emit `release_state_changed` only when the effective state actually moved.
pub(crate) async fn emit_if_state_changed(
    events: &Arc<dyn EventPublisher>,
    team_id: Uuid,
    release_id: Uuid,
    old: ReleaseState,
    new: ReleaseState,
) {
    if old != new {
        events
            .publish(DomainEvent::ReleaseStateChanged {
                team_id,
                release_id,
                new_state: new,
            })
            .await;
    }
}

/// Snapshot the effective state of every release linked to an incident, *before*
/// the incident's status change is persisted. Paired with
/// `emit_release_state_changes` after the persist to detect auto-(un)blocks.
pub(crate) async fn snapshot_linked_releases(
    releases: &Arc<dyn ReleaseRepo>,
    incident_id: Uuid,
) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState, ReleaseState)>, DomainError> {
    let linked = releases
        .list_release_states_linked_to_incident(incident_id)
        .await?;
    let mut out = Vec::with_capacity(linked.len());
    for (release_id, team_id, base) in linked {
        let has_active = releases.count_active_linked_incidents(release_id).await? > 0;
        out.push((
            release_id,
            team_id,
            base,
            effective_release_state(base, has_active),
        ));
    }
    Ok(out)
}

/// After an incident status change is persisted, recompute the snapshotted
/// releases and emit `release_state_changed` for any whose effective state moved.
pub(crate) async fn emit_release_state_changes(
    releases: &Arc<dyn ReleaseRepo>,
    events: &Arc<dyn EventPublisher>,
    snapshot: Vec<(Uuid, Uuid, ReleaseBaseState, ReleaseState)>,
) -> Result<(), DomainError> {
    for (release_id, team_id, base, old_eff) in snapshot {
        let has_active = releases.count_active_linked_incidents(release_id).await? > 0;
        let new_eff = effective_release_state(base, has_active);
        emit_if_state_changed(events, team_id, release_id, old_eff, new_eff).await;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::release::{Release, ReleaseBaseState};
    use crate::ports::ReleaseRepo;

    /// In-memory release repo for use-case tests. `active_incidents` marks which
    /// linked incident ids count as active; `scripted_counts` optionally overrides
    /// `count_active_linked_incidents` per release (a queue popped per call) so a
    /// test can drive an auto-unblock deterministically.
    #[derive(Default)]
    pub struct MockReleaseRepo {
        pub releases: Mutex<HashMap<Uuid, Release>>,
        pub links: Mutex<Vec<(Uuid, Uuid)>>,
        pub active_incidents: Mutex<HashSet<Uuid>>,
        pub scripted_counts: Mutex<HashMap<Uuid, VecDeque<u64>>>,
        pub reject_update: bool,
    }

    impl MockReleaseRepo {
        pub fn seed_release(&self, release: Release) {
            self.releases.lock().unwrap().insert(release.id, release);
        }
        pub fn mark_active(&self, incident_id: Uuid) {
            self.active_incidents.lock().unwrap().insert(incident_id);
        }
        pub fn script_count(&self, release_id: Uuid, counts: Vec<u64>) {
            self.scripted_counts
                .lock()
                .unwrap()
                .insert(release_id, counts.into());
        }
        pub fn reject_updates() -> Self {
            Self {
                reject_update: true,
                ..Self::default()
            }
        }
    }

    #[async_trait]
    impl ReleaseRepo for MockReleaseRepo {
        async fn save_release(&self, release: &Release) -> Result<(), DomainError> {
            self.seed_release(release.clone());
            Ok(())
        }

        async fn create_release(
            &self,
            release: &Release,
            _delivery_id: &str,
            _event: &crate::domain::automation::ExternalEvent,
        ) -> Result<(), DomainError> {
            self.save_release(release).await
        }

        async fn create_blocking_incident(
            &self,
            release_id: Uuid,
            _expected_updated_at: chrono::DateTime<chrono::Utc>,
            incident: &crate::domain::incident::Incident,
            _event: &crate::domain::incident_event::IncidentEvent,
        ) -> Result<(), DomainError> {
            self.link_incident(release_id, incident.id).await
        }

        async fn find_release_by_id(
            &self,
            release_id: Uuid,
        ) -> Result<Option<Release>, DomainError> {
            Ok(self.releases.lock().unwrap().get(&release_id).cloned())
        }

        async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError> {
            Ok(self
                .releases
                .lock()
                .unwrap()
                .values()
                .filter(|r| r.team_id == team_id)
                .cloned()
                .collect())
        }

        async fn update_release(
            &self,
            release: &Release,
            _expected_updated_at: chrono::DateTime<chrono::Utc>,
        ) -> Result<(), DomainError> {
            if self.reject_update {
                return Err(DomainError::ConcurrentModification);
            }
            self.seed_release(release.clone());
            Ok(())
        }

        async fn link_incident(
            &self,
            release_id: Uuid,
            incident_id: Uuid,
        ) -> Result<(), DomainError> {
            let mut links = self.links.lock().unwrap();
            if !links.contains(&(release_id, incident_id)) {
                links.push((release_id, incident_id));
                if let Some(release) = self.releases.lock().unwrap().get_mut(&release_id) {
                    release.updated_at = Utc::now();
                }
            }
            Ok(())
        }

        async fn unlink_incident(
            &self,
            release_id: Uuid,
            incident_id: Uuid,
        ) -> Result<(), DomainError> {
            let mut links = self.links.lock().unwrap();
            let before = links.len();
            links.retain(|pair| *pair != (release_id, incident_id));
            if links.len() != before {
                if let Some(release) = self.releases.lock().unwrap().get_mut(&release_id) {
                    release.updated_at = Utc::now();
                }
            }
            Ok(())
        }

        async fn list_linked_incident_ids(
            &self,
            release_id: Uuid,
        ) -> Result<Vec<Uuid>, DomainError> {
            Ok(self
                .links
                .lock()
                .unwrap()
                .iter()
                .filter(|(r, _)| *r == release_id)
                .map(|(_, i)| *i)
                .collect())
        }

        async fn count_active_linked_incidents(
            &self,
            release_id: Uuid,
        ) -> Result<u64, DomainError> {
            if let Some(queue) = self.scripted_counts.lock().unwrap().get_mut(&release_id) {
                if let Some(n) = queue.pop_front() {
                    return Ok(n);
                }
            }
            let active = self.active_incidents.lock().unwrap();
            Ok(self
                .links
                .lock()
                .unwrap()
                .iter()
                .filter(|(r, i)| *r == release_id && active.contains(i))
                .count() as u64)
        }

        async fn list_release_states_linked_to_incident(
            &self,
            incident_id: Uuid,
        ) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState)>, DomainError> {
            let releases = self.releases.lock().unwrap();
            Ok(self
                .links
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, i)| *i == incident_id)
                .filter_map(|(r, _)| releases.get(r).map(|rel| (*r, rel.team_id, rel.base_state)))
                .collect())
        }
    }
}
