use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    Open,
    Acknowledged,
    Escalated,
    Resolved,
}

impl fmt::Display for IncidentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            IncidentStatus::Open => "open",
            IncidentStatus::Acknowledged => "acknowledged",
            IncidentStatus::Escalated => "escalated",
            IncidentStatus::Resolved => "resolved",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incident {
    pub id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: IncidentStatus,
    pub severity: Severity,
    pub assignee: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Incident {
    pub fn new(
        team_id: Uuid,
        title: impl Into<String>,
        severity: Severity,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(DomainError::InvalidIncidentTitle);
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            team_id,
            title,
            description: String::new(),
            status: IncidentStatus::Open,
            severity,
            assignee: None,
            created_by: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn new_by(
        team_id: Uuid,
        title: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
        created_by: Uuid,
    ) -> Result<Self, DomainError> {
        let mut incident = Self::new(team_id, title, severity)?;
        incident.description = description.into().trim().to_string();
        incident.created_by = Some(created_by);
        Ok(incident)
    }

    /// System-created incidents (for example automation reactions) have no
    /// human actor but may still carry normalized operational context.
    pub fn new_with_description(
        team_id: Uuid,
        title: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
    ) -> Result<Self, DomainError> {
        let mut incident = Self::new(team_id, title, severity)?;
        incident.description = description.into().trim().to_string();
        Ok(incident)
    }

    /// Assign a responder. Idempotent: returns `true` only when the assignee
    /// actually changed, so the caller can skip a redundant persistence write.
    pub fn assign(&mut self, user_id: Uuid) -> bool {
        let changed = self.assignee != Some(user_id);
        self.assignee = Some(user_id);
        if changed {
            self.updated_at = Utc::now();
        }
        changed
    }

    pub fn acknowledge(&mut self) -> Result<bool, DomainError> {
        match self.status {
            IncidentStatus::Open => {
                self.status = IncidentStatus::Acknowledged;
                self.updated_at = Utc::now();
                Ok(true)
            }
            IncidentStatus::Acknowledged => Ok(false),
            IncidentStatus::Escalated | IncidentStatus::Resolved => {
                Err(DomainError::InvalidIncidentTransition)
            }
        }
    }

    pub fn escalate(&mut self) -> Result<bool, DomainError> {
        match self.status {
            IncidentStatus::Acknowledged => {
                self.status = IncidentStatus::Escalated;
                self.updated_at = Utc::now();
                Ok(true)
            }
            IncidentStatus::Escalated => Ok(false),
            IncidentStatus::Open | IncidentStatus::Resolved => {
                Err(DomainError::InvalidIncidentTransition)
            }
        }
    }

    pub fn resolve(&mut self) -> Result<bool, DomainError> {
        match self.status {
            IncidentStatus::Acknowledged | IncidentStatus::Escalated => {
                self.status = IncidentStatus::Resolved;
                self.updated_at = Utc::now();
                Ok(true)
            }
            IncidentStatus::Resolved => Ok(false),
            IncidentStatus::Open => Err(DomainError::InvalidIncidentTransition),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_incident_starts_open_with_given_severity() {
        let team_id = Uuid::new_v4();
        let incident = Incident::new(team_id, "Primary DB latency", Severity::High).unwrap();

        assert_eq!(incident.team_id, team_id);
        assert_eq!(incident.title, "Primary DB latency");
        assert_eq!(incident.status, IncidentStatus::Open);
        assert_eq!(incident.severity, Severity::High);
    }

    #[test]
    fn blank_title_is_rejected() {
        let result = Incident::new(Uuid::new_v4(), "   ", Severity::Low);

        assert_eq!(result.unwrap_err(), DomainError::InvalidIncidentTitle);
    }

    #[test]
    fn lifecycle_follows_open_ack_escalated_resolved() {
        let mut incident =
            Incident::new(Uuid::new_v4(), "Cache outage", Severity::Critical).unwrap();

        assert!(incident.acknowledge().unwrap());
        assert!(incident.escalate().unwrap());
        assert!(incident.resolve().unwrap());
        assert_eq!(incident.status, IncidentStatus::Resolved);
    }

    #[test]
    fn resolve_is_idempotent_once_resolved() {
        let mut incident = Incident::new(Uuid::new_v4(), "API errors", Severity::High).unwrap();
        incident.acknowledge().unwrap();
        incident.resolve().unwrap();

        let changed = incident.resolve().unwrap();

        assert!(!changed);
        assert_eq!(incident.status, IncidentStatus::Resolved);
    }

    #[test]
    fn invalid_jump_from_open_to_resolved_is_rejected() {
        let mut incident = Incident::new(Uuid::new_v4(), "Queue stall", Severity::Medium).unwrap();

        let result = incident.resolve();

        assert_eq!(result.unwrap_err(), DomainError::InvalidIncidentTransition);
    }

    #[test]
    fn escalating_before_acknowledge_is_rejected() {
        let mut incident = Incident::new(Uuid::new_v4(), "Disk pressure", Severity::High).unwrap();

        let result = incident.escalate();

        assert_eq!(result.unwrap_err(), DomainError::InvalidIncidentTransition);
    }

    #[test]
    fn assign_sets_assignee_and_reports_change() {
        let mut incident = Incident::new(Uuid::new_v4(), "Queue stall", Severity::Low).unwrap();
        let responder = Uuid::new_v4();

        assert!(incident.assignee.is_none());
        assert!(incident.assign(responder));
        assert_eq!(incident.assignee, Some(responder));
    }

    #[test]
    fn assign_same_user_is_idempotent() {
        let mut incident = Incident::new(Uuid::new_v4(), "Queue stall", Severity::Low).unwrap();
        let responder = Uuid::new_v4();
        incident.assign(responder);

        let changed = incident.assign(responder);

        assert!(!changed);
        assert_eq!(incident.assignee, Some(responder));
    }
}
