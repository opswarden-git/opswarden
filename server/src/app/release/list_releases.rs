// --- server/src/app/release/list_releases.rs ---

use std::{collections::HashMap, sync::Arc};

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{IncidentStatus, Severity};
use crate::domain::release::ReleaseStep;
use crate::ports::{IncidentRepo, ReleaseRepo, TeamRepo};

use super::{load_detail, ReleaseDetail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBlocker {
    pub incident_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub severity: Severity,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReleaseListItem {
    pub detail: ReleaseDetail,
    pub completed_steps: usize,
    pub total_steps: usize,
    pub next_step: Option<ReleaseStep>,
    pub blockers: Vec<ReleaseBlocker>,
}

pub struct ListReleasesCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
}

pub struct ListReleasesUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
    releases: Arc<dyn ReleaseRepo>,
}

impl ListReleasesUseCase {
    pub fn new(
        teams: Arc<dyn TeamRepo>,
        incidents: Arc<dyn IncidentRepo>,
        releases: Arc<dyn ReleaseRepo>,
    ) -> Self {
        Self {
            teams,
            incidents,
            releases,
        }
    }

    pub async fn list(
        &self,
        cmd: ListReleasesCommand,
    ) -> Result<Vec<ReleaseListItem>, DomainError> {
        // Any member (Observer included) may read; non-members are forbidden.
        self.teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let incidents_by_id: HashMap<_, _> = self
            .incidents
            .list_incidents_for_team(cmd.team_id)
            .await?
            .into_iter()
            .map(|incident| (incident.id, incident))
            .collect();
        let releases = self.releases.list_releases_for_team(cmd.team_id).await?;
        let mut out = Vec::with_capacity(releases.len());
        for release in releases {
            let completed_steps = release
                .steps
                .iter()
                .filter(|step| step.is_validated())
                .count();
            let total_steps = release.steps.len();
            let next_step = release
                .steps
                .iter()
                .find(|step| !step.is_validated())
                .cloned();
            let detail = load_detail(&self.releases, release).await?;
            let blockers =
                if detail.effective_state == crate::domain::release::ReleaseState::Blocked {
                    detail
                        .linked_incident_ids
                        .iter()
                        .filter_map(|incident_id| incidents_by_id.get(incident_id))
                        .filter(|incident| incident.status != IncidentStatus::Resolved)
                        .map(|incident| ReleaseBlocker {
                            incident_id: incident.id,
                            title: incident.title.clone(),
                            status: incident.status,
                            severity: incident.severity,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
            out.push(ReleaseListItem {
                detail,
                completed_steps,
                total_steps,
                next_step,
                blockers,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::app::release::tests::MockReleaseRepo;
    use crate::domain::incident::{Incident, Severity};
    use crate::domain::release::{Release, ReleaseState};
    use crate::domain::team::Role;
    use crate::ports::ReleaseRepo;

    #[tokio::test]
    async fn an_observer_can_list_releases() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(Release::new(team_id, "v1", vec!["build".into()]).unwrap());
        let uc = ListReleasesUseCase::new(teams, incidents, releases);

        let out = uc
            .list(ListReleasesCommand {
                team_id,
                requester_id: requester,
            })
            .await
            .unwrap();

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].completed_steps, 0);
        assert_eq!(out[0].total_steps, 1);
        assert_eq!(out[0].next_step.as_ref().map(|step| step.position), Some(0));
    }

    #[tokio::test]
    async fn list_explains_progress_and_human_blockers() {
        let team_id = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let mut release =
            Release::new(team_id, "v2", vec!["build".into(), "production".into()]).unwrap();
        release.validate_step("build", requester, false).unwrap();
        let release_id = release.id;
        let incident =
            Incident::new(team_id, "Production smoke tests failing", Severity::High).unwrap();
        let incident_id = incident.id;
        let teams =
            Arc::new(MockTeamRepo::default().with_member(team_id, requester, Role::Responder));
        let incidents = Arc::new(MockIncidentRepo::with_incident(incident));
        let releases = Arc::new(MockReleaseRepo::default());
        releases.seed_release(release);
        releases
            .link_incident(release_id, incident_id)
            .await
            .unwrap();
        releases.mark_active(incident_id);
        let uc = ListReleasesUseCase::new(teams, incidents, releases);

        let out = uc
            .list(ListReleasesCommand {
                team_id,
                requester_id: requester,
            })
            .await
            .unwrap();

        let item = &out[0];
        assert_eq!(item.detail.effective_state, ReleaseState::Blocked);
        assert_eq!((item.completed_steps, item.total_steps), (1, 2));
        assert_eq!(
            item.next_step.as_ref().map(|step| step.name.as_str()),
            Some("production")
        );
        assert_eq!(item.blockers.len(), 1);
        assert_eq!(item.blockers[0].incident_id, incident_id);
        assert_eq!(item.blockers[0].title, "Production smoke tests failing");
    }

    #[tokio::test]
    async fn a_non_member_cannot_list_releases() {
        let teams = Arc::new(MockTeamRepo::default());
        let incidents = Arc::new(MockIncidentRepo::default());
        let releases = Arc::new(MockReleaseRepo::default());
        let uc = ListReleasesUseCase::new(teams, incidents, releases);

        let err = uc
            .list(ListReleasesCommand {
                team_id: Uuid::new_v4(),
                requester_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }
}
