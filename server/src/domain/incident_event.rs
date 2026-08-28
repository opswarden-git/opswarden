use std::fmt;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use super::incident::{Incident, IncidentStatus, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentEventKind {
    Created,
    StatusChanged,
    Assigned,
    SeverityChanged,
    ReleaseStepValidated,
}

impl fmt::Display for IncidentEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Created => "created",
            Self::StatusChanged => "status_changed",
            Self::Assigned => "assigned",
            Self::SeverityChanged => "severity_changed",
            Self::ReleaseStepValidated => "release_step_validated",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncidentEvent {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub kind: IncidentEventKind,
    pub actor_id: Option<Uuid>,
    pub data: Value,
    pub created_at: DateTime<Utc>,
}

impl IncidentEvent {
    pub fn created(incident: &Incident, actor_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            incident_id: incident.id,
            kind: IncidentEventKind::Created,
            actor_id,
            data: json!({
                "status": incident.status.to_string(),
                "severity": incident.severity.to_string(),
            }),
            created_at: incident.created_at,
        }
    }

    pub fn status_changed(
        incident_id: Uuid,
        actor_id: Uuid,
        from: IncidentStatus,
        to: IncidentStatus,
    ) -> Self {
        Self::new(
            incident_id,
            IncidentEventKind::StatusChanged,
            actor_id,
            json!({ "from": from.to_string(), "to": to.to_string() }),
        )
    }

    pub fn assigned(incident_id: Uuid, actor_id: Uuid, assignee_id: Uuid) -> Self {
        Self::new(
            incident_id,
            IncidentEventKind::Assigned,
            actor_id,
            json!({ "assignee_id": assignee_id }),
        )
    }

    pub fn severity_changed(
        incident_id: Uuid,
        actor_id: Uuid,
        from: Severity,
        to: Severity,
    ) -> Self {
        Self::new(
            incident_id,
            IncidentEventKind::SeverityChanged,
            actor_id,
            json!({ "from": from.to_string(), "to": to.to_string() }),
        )
    }

    /// A release this incident blocks moved a step forward. Recorded on the
    /// incident so its war room reads one history, not two: the release title
    /// is snapshotted because an audit line must not change when it is renamed.
    pub fn release_step_validated(
        incident_id: Uuid,
        actor_id: Uuid,
        release_id: Uuid,
        release_title: &str,
        step: &str,
    ) -> Self {
        Self::new(
            incident_id,
            IncidentEventKind::ReleaseStepValidated,
            actor_id,
            json!({
                "release_id": release_id,
                "release_title": release_title,
                "step": step,
            }),
        )
    }

    fn new(incident_id: Uuid, kind: IncidentEventKind, actor_id: Uuid, data: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            incident_id,
            kind,
            actor_id: Some(actor_id),
            data,
            created_at: Utc::now(),
        }
    }
}
