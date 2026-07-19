// --- server/src/domain/release.rs ---
//
// A Release: a planned deployment composed of ordered, sequentially-validated
// steps. An active linked Incident blocks an in-progress release until resolved.
//
// Design (fixed): `blocked` is a *derived* effective state, never stored. The
// stored `base_state` is one of created/in_progress/completed/cancelled; the
// effective state is `blocked` iff base is `in_progress` and at least one linked
// incident is still active. Whether incidents are active is a use-case concern
// (it queries the repo), so the domain takes that as a plain `has_active` bool
// and stays free of any incident or persistence knowledge.

use std::fmt;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::error::DomainError;

pub const MAX_RELEASE_TITLE_LEN: usize = 200;
pub const MAX_RELEASE_STEPS: usize = 50;
pub const MAX_STEP_NAME_LEN: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseState {
    Created,
    InProgress,
    /// Effective-only: never persisted, computed from active linked incidents.
    Blocked,
    Completed,
    Cancelled,
}

impl fmt::Display for ReleaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ReleaseState::Created => "created",
            ReleaseState::InProgress => "in_progress",
            ReleaseState::Blocked => "blocked",
            ReleaseState::Completed => "completed",
            ReleaseState::Cancelled => "cancelled",
        };
        f.write_str(value)
    }
}

impl ReleaseState {
    /// Parse a *stored* base state. `blocked` is never stored, so it (and any
    /// unknown value) is rejected — the adapter treats it as a storage fault.
    pub fn from_base_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "created" => Ok(ReleaseState::Created),
            "in_progress" => Ok(ReleaseState::InProgress),
            "completed" => Ok(ReleaseState::Completed),
            "cancelled" => Ok(ReleaseState::Cancelled),
            _ => Err(DomainError::Storage),
        }
    }
}

/// The effective state given the stored base and whether a blocking incident is
/// active. Pure and total — the single source of truth for "is it blocked?".
pub fn effective_release_state(base: ReleaseState, has_active_incident: bool) -> ReleaseState {
    if base == ReleaseState::InProgress && has_active_incident {
        ReleaseState::Blocked
    } else {
        base
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseStep {
    pub position: i32,
    pub name: String,
    pub validated_by: Option<Uuid>,
    pub validated_at: Option<DateTime<Utc>>,
}

impl ReleaseStep {
    pub fn is_validated(&self) -> bool {
        self.validated_at.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub id: Uuid,
    pub team_id: Uuid,
    pub title: String,
    /// Stored lifecycle, never `Blocked` (that is computed via `effective_state`).
    pub base_state: ReleaseState,
    pub steps: Vec<ReleaseStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Release {
    /// Build a release from a non-empty title and an ordered list of distinct,
    /// non-blank step names. Starts in `created` with no step validated.
    pub fn new(
        team_id: Uuid,
        title: impl Into<String>,
        step_names: Vec<String>,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        let trimmed = title.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_RELEASE_TITLE_LEN {
            return Err(DomainError::InvalidReleaseTitle);
        }

        if step_names.is_empty() || step_names.len() > MAX_RELEASE_STEPS {
            return Err(DomainError::InvalidReleaseSteps);
        }
        let mut steps = Vec::with_capacity(step_names.len());
        let mut seen = std::collections::HashSet::new();
        for (i, raw) in step_names.iter().enumerate() {
            let name = raw.trim();
            if name.is_empty() || name.len() > MAX_STEP_NAME_LEN || !seen.insert(name.to_string()) {
                return Err(DomainError::InvalidReleaseSteps);
            }
            steps.push(ReleaseStep {
                position: i as i32,
                name: name.to_string(),
                validated_by: None,
                validated_at: None,
            });
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            team_id,
            title: trimmed.to_string(),
            base_state: ReleaseState::Created,
            steps,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn effective_state(&self, has_active_incident: bool) -> ReleaseState {
        effective_release_state(self.base_state, has_active_incident)
    }

    /// Validate the next step in sequence. Allowed only while the release is not
    /// terminal and not effectively blocked; the requested `step_name` must be
    /// exactly the next unvalidated step. Promotes `created → in_progress` on the
    /// first validation and `→ completed` once every step is validated.
    pub fn validate_step(
        &mut self,
        step_name: &str,
        by: Uuid,
        has_active_incident: bool,
    ) -> Result<(), DomainError> {
        match self.base_state {
            ReleaseState::Completed | ReleaseState::Cancelled => {
                return Err(DomainError::InvalidReleaseTransition);
            }
            // `Blocked` is never a stored base; only Created/InProgress remain.
            _ => {}
        }

        if self.effective_state(has_active_incident) == ReleaseState::Blocked {
            return Err(DomainError::ReleaseBlocked);
        }

        let next = self
            .steps
            .iter()
            .position(|step| !step.is_validated())
            .ok_or(DomainError::InvalidReleaseTransition)?;
        if self.steps[next].name != step_name {
            // Unknown step, already-validated step, or out-of-order step.
            return Err(DomainError::InvalidReleaseStep);
        }

        self.steps[next].validated_by = Some(by);
        self.steps[next].validated_at = Some(Utc::now());

        if self.base_state == ReleaseState::Created {
            self.base_state = ReleaseState::InProgress;
        }
        if self.steps.iter().all(ReleaseStep::is_validated) {
            self.base_state = ReleaseState::Completed;
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Cancel the release. Terminal states cannot be cancelled.
    pub fn cancel(&mut self) -> Result<(), DomainError> {
        match self.base_state {
            ReleaseState::Completed | ReleaseState::Cancelled => {
                Err(DomainError::InvalidReleaseTransition)
            }
            _ => {
                self.base_state = ReleaseState::Cancelled;
                self.updated_at = Utc::now();
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        Release::new(
            Uuid::new_v4(),
            "v1.2.0",
            vec!["build".into(), "staging".into(), "production".into()],
        )
        .unwrap()
    }

    #[test]
    fn new_release_starts_created_with_unvalidated_steps() {
        let r = release();
        assert_eq!(r.base_state, ReleaseState::Created);
        assert_eq!(r.steps.len(), 3);
        assert!(r.steps.iter().all(|s| !s.is_validated()));
    }

    #[test]
    fn blank_title_and_empty_or_dup_steps_are_rejected() {
        assert_eq!(
            Release::new(Uuid::new_v4(), "  ", vec!["a".into()]).unwrap_err(),
            DomainError::InvalidReleaseTitle
        );
        assert_eq!(
            Release::new(Uuid::new_v4(), "ok", vec![]).unwrap_err(),
            DomainError::InvalidReleaseSteps
        );
        assert_eq!(
            Release::new(Uuid::new_v4(), "ok", vec!["a".into(), "a".into()]).unwrap_err(),
            DomainError::InvalidReleaseSteps
        );
        assert_eq!(
            Release::new(Uuid::new_v4(), "ok", vec!["  ".into()]).unwrap_err(),
            DomainError::InvalidReleaseSteps
        );
    }

    #[test]
    fn effective_state_is_blocked_only_when_in_progress_with_active_incident() {
        assert_eq!(
            effective_release_state(ReleaseState::Created, true),
            ReleaseState::Created
        );
        assert_eq!(
            effective_release_state(ReleaseState::InProgress, true),
            ReleaseState::Blocked
        );
        assert_eq!(
            effective_release_state(ReleaseState::InProgress, false),
            ReleaseState::InProgress
        );
        assert_eq!(
            effective_release_state(ReleaseState::Completed, true),
            ReleaseState::Completed
        );
    }

    #[test]
    fn steps_validate_sequentially_and_promote_then_complete() {
        let mut r = release();
        let by = Uuid::new_v4();
        let stale = r.updated_at - chrono::Duration::seconds(1);
        r.updated_at = stale;

        r.validate_step("build", by, false).unwrap();
        assert_eq!(r.base_state, ReleaseState::InProgress);
        assert!(r.steps[0].is_validated());
        assert_eq!(r.steps[0].validated_by, Some(by));
        assert!(r.updated_at > stale);

        r.validate_step("staging", by, false).unwrap();
        r.validate_step("production", by, false).unwrap();
        assert_eq!(r.base_state, ReleaseState::Completed);
    }

    #[test]
    fn out_of_order_or_unknown_step_is_rejected() {
        let mut r = release();
        assert_eq!(
            r.validate_step("production", Uuid::new_v4(), false)
                .unwrap_err(),
            DomainError::InvalidReleaseStep
        );
        assert_eq!(
            r.validate_step("nope", Uuid::new_v4(), false).unwrap_err(),
            DomainError::InvalidReleaseStep
        );
    }

    #[test]
    fn validating_a_blocked_release_is_refused() {
        let mut r = release();
        r.validate_step("build", Uuid::new_v4(), false).unwrap(); // now in_progress
                                                                  // in_progress + active incident => blocked
        assert_eq!(
            r.validate_step("staging", Uuid::new_v4(), true)
                .unwrap_err(),
            DomainError::ReleaseBlocked
        );
    }

    #[test]
    fn validating_a_terminal_release_is_refused() {
        let mut r = release();
        r.cancel().unwrap();
        assert_eq!(
            r.validate_step("build", Uuid::new_v4(), false).unwrap_err(),
            DomainError::InvalidReleaseTransition
        );
    }

    #[test]
    fn cancel_is_refused_on_terminal_states() {
        let mut r = release();
        let stale = r.updated_at - chrono::Duration::seconds(1);
        r.updated_at = stale;
        r.cancel().unwrap();
        assert_eq!(r.base_state, ReleaseState::Cancelled);
        assert!(r.updated_at > stale);
        assert_eq!(
            r.cancel().unwrap_err(),
            DomainError::InvalidReleaseTransition
        );
    }
}
