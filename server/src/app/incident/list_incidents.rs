//
// Read use-case: the operational incident queue of a team. RBAC: every team
// member may read it. The projection combines incidents with the team roster,
// then applies deterministic filters and sorting before returning the result.

use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::incident::{Incident, IncidentStatus, Severity};
use crate::domain::team::TeamMemberView;
use crate::ports::{IncidentRepo, TeamRepo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IncidentAssigneeFilter {
    #[default]
    Any,
    Unassigned,
    User(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IncidentSort {
    #[default]
    Newest,
    Oldest,
    Severity,
}

pub struct ListIncidentsCommand {
    pub team_id: Uuid,
    pub requester_id: Uuid,
    pub status: Option<IncidentStatus>,
    pub severity: Option<Severity>,
    pub assignee: IncidentAssigneeFilter,
    pub query: Option<String>,
    pub sort: IncidentSort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentListItem {
    pub incident: Incident,
    pub assignee: Option<TeamMemberView>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IncidentCounts {
    pub all: u64,
    pub open: u64,
    pub acknowledged: u64,
    pub escalated: u64,
    pub resolved: u64,
}

impl IncidentCounts {
    fn from_incidents(incidents: &[Incident]) -> Self {
        let mut counts = Self {
            all: incidents.len() as u64,
            ..Self::default()
        };

        for incident in incidents {
            match incident.status {
                IncidentStatus::Open => counts.open += 1,
                IncidentStatus::Acknowledged => counts.acknowledged += 1,
                IncidentStatus::Escalated => counts.escalated += 1,
                IncidentStatus::Resolved => counts.resolved += 1,
            }
        }
        counts
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListIncidentsResult {
    pub items: Vec<IncidentListItem>,
    pub counts: IncidentCounts,
}

pub struct ListIncidentsUseCase {
    teams: Arc<dyn TeamRepo>,
    incidents: Arc<dyn IncidentRepo>,
}

impl ListIncidentsUseCase {
    pub fn new(teams: Arc<dyn TeamRepo>, incidents: Arc<dyn IncidentRepo>) -> Self {
        Self { teams, incidents }
    }

    pub async fn list_incidents(
        &self,
        cmd: ListIncidentsCommand,
    ) -> Result<ListIncidentsResult, DomainError> {
        self.teams
            .find_member_role(cmd.team_id, cmd.requester_id)
            .await?
            .ok_or(DomainError::Forbidden)?;

        let incidents = self.incidents.list_incidents_for_team(cmd.team_id).await?;
        let counts = IncidentCounts::from_incidents(&incidents);
        let members: HashMap<_, _> = self
            .teams
            .list_members(cmd.team_id)
            .await?
            .into_iter()
            .map(|member| (member.user_id, member))
            .collect();

        let query = cmd
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(str::to_lowercase);

        let mut items: Vec<_> = incidents
            .into_iter()
            .map(|incident| IncidentListItem {
                assignee: incident.assignee.and_then(|id| members.get(&id).cloned()),
                incident,
            })
            .filter(|item| {
                cmd.status
                    .is_none_or(|status| item.incident.status == status)
            })
            .filter(|item| {
                cmd.severity
                    .is_none_or(|severity| item.incident.severity == severity)
            })
            .filter(|item| match cmd.assignee {
                IncidentAssigneeFilter::Any => true,
                IncidentAssigneeFilter::Unassigned => item.incident.assignee.is_none(),
                IncidentAssigneeFilter::User(user_id) => item.incident.assignee == Some(user_id),
            })
            .filter(|item| {
                query.as_ref().is_none_or(|query| {
                    item.incident.title.to_lowercase().contains(query)
                        || item.incident.id.to_string().contains(query)
                        || item
                            .assignee
                            .as_ref()
                            .is_some_and(|assignee| assignee.email.to_lowercase().contains(query))
                })
            })
            .collect();

        items.sort_by(|left, right| match cmd.sort {
            IncidentSort::Newest => right
                .incident
                .created_at
                .cmp(&left.incident.created_at)
                .then_with(|| right.incident.id.cmp(&left.incident.id)),
            IncidentSort::Oldest => left
                .incident
                .created_at
                .cmp(&right.incident.created_at)
                .then_with(|| left.incident.id.cmp(&right.incident.id)),
            IncidentSort::Severity => severity_rank(right.incident.severity)
                .cmp(&severity_rank(left.incident.severity))
                .then_with(|| right.incident.created_at.cmp(&left.incident.created_at))
                .then_with(|| right.incident.id.cmp(&left.incident.id)),
        });

        Ok(ListIncidentsResult { items, counts })
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Low => 0,
        Severity::Medium => 1,
        Severity::High => 2,
        Severity::Critical => 3,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    use crate::app::incident::tests::{MockIncidentRepo, MockTeamRepo};
    use crate::domain::team::Role;

    fn command(team_id: Uuid, requester_id: Uuid) -> ListIncidentsCommand {
        ListIncidentsCommand {
            team_id,
            requester_id,
            status: None,
            severity: None,
            assignee: IncidentAssigneeFilter::Any,
            query: None,
            sort: IncidentSort::Newest,
        }
    }

    #[tokio::test]
    async fn member_gets_counts_and_human_assignee_identity() {
        let team = Uuid::new_v4();
        let requester = Uuid::new_v4();
        let responder = Uuid::new_v4();
        let mut assigned = Incident::new(team, "DB latency", Severity::High).unwrap();
        assigned.assign(responder);
        let mut resolved = Incident::new(team, "Cache recovered", Severity::Low).unwrap();
        resolved.status = IncidentStatus::Resolved;
        let teams = Arc::new(
            MockTeamRepo::default()
                .with_member(team, requester, Role::Observer)
                .with_member(team, responder, Role::Responder),
        );
        let incidents = Arc::new(MockIncidentRepo::with_incidents(vec![assigned, resolved]));
        let use_case = ListIncidentsUseCase::new(teams, incidents);

        let result = use_case
            .list_incidents(command(team, requester))
            .await
            .unwrap();

        assert_eq!(result.counts.all, 2);
        assert_eq!(result.counts.open, 1);
        assert_eq!(result.counts.resolved, 1);
        let assignee = result
            .items
            .iter()
            .find_map(|item| item.assignee.as_ref())
            .unwrap();
        assert_eq!(assignee.user_id, responder);
        assert!(assignee.email.ends_with("@test.local"));
    }

    #[tokio::test]
    async fn filters_searches_and_sorts_the_queue() {
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let now = Utc::now();
        let mut older = Incident::new(team, "Database saturation", Severity::Critical).unwrap();
        older.created_at = now - Duration::hours(2);
        let mut newer = Incident::new(team, "Database replica lag", Severity::High).unwrap();
        newer.created_at = now;
        let other = Incident::new(team, "API timeout", Severity::Critical).unwrap();
        let teams = Arc::new(MockTeamRepo::default().with_member(team, user, Role::Observer));
        let incidents = Arc::new(MockIncidentRepo::with_incidents(vec![
            older.clone(),
            newer,
            other,
        ]));
        let use_case = ListIncidentsUseCase::new(teams, incidents);
        let mut cmd = command(team, user);
        cmd.query = Some("database".into());
        cmd.sort = IncidentSort::Severity;

        let result = use_case.list_incidents(cmd).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].incident.id, older.id);
    }

    #[tokio::test]
    async fn non_member_is_forbidden() {
        let team = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let teams = Arc::new(MockTeamRepo::default());
        let incidents = Arc::new(MockIncidentRepo::default());
        let use_case = ListIncidentsUseCase::new(teams, incidents);

        let err = use_case
            .list_incidents(command(team, outsider))
            .await
            .unwrap_err();

        assert_eq!(err, DomainError::Forbidden);
    }
}
