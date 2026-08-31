// --- server/src/domain/team.rs ---

use chrono::{DateTime, Utc};
use rand::RngExt;
use std::fmt;
use uuid::Uuid;

use super::error::DomainError;

/// RBAC roles inside a team, ordered from least to most privileged.
/// The ordering powers `can_act_as`: a higher role satisfies any lower
/// requirement (Manager ⊇ Responder ⊇ Observer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Observer,
    Responder,
    Manager,
}

impl Role {
    pub const ALL: &'static [Self] = &[Self::Observer, Self::Responder, Self::Manager];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observer => "observer",
            Self::Responder => "responder",
            Self::Manager => "manager",
        }
    }

    /// Privilege rank; only meaningful relative to other ranks.
    fn rank(self) -> u8 {
        match self {
            Role::Observer => 0,
            Role::Responder => 1,
            Role::Manager => 2,
        }
    }

    /// True when `self` is allowed to perform an action requiring `required`.
    pub fn can_act_as(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }
}

impl TryFrom<&str> for Role {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "observer" => Ok(Self::Observer),
            "responder" => Ok(Self::Responder),
            "manager" => Ok(Self::Manager),
            _ => Err(DomainError::InvalidRole),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Human-friendly, dictatable invitation code: `OPS-` + 6 chars drawn from an
/// alphabet that excludes look-alikes (0/O, 1/I/L) to survive being read aloud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationCode(String);

const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const CODE_LEN: usize = 6;

impl InvitationCode {
    /// Generate a fresh random code, e.g. `OPS-A7B9X2`.
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let suffix: String = (0..CODE_LEN)
            .map(|_| CODE_ALPHABET[rng.random_range(0..CODE_ALPHABET.len())] as char)
            .collect();
        Self(format!("OPS-{suffix}"))
    }

    /// Rehydrate a code already persisted (no validation: the source is trusted).
    pub fn from_existing(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A team aggregate root. Membership lives in `TeamMember` rows, not here, so a
/// `Team` stays a small, persistable identity + its invitation handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub invitation_code: InvitationCode,
    pub created_at: DateTime<Utc>,
}

impl Team {
    /// Create a team with a fresh id and invitation code. The name is rejected
    /// when empty (after trimming) to keep the aggregate always valid.
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::InvalidTeamName);
        }
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            invitation_code: InvitationCode::generate(),
            created_at: Utc::now(),
        })
    }
}

/// The association of a user with a team under a given role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

/// Read projection of a team member enriched with the user's email, for the
/// roster view. Joins `team_members` with `users`, so it carries identity the
/// bare `TeamMember` association does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMemberView {
    pub user_id: Uuid,
    pub email: String,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

/// Directory projection for one team the current user belongs to. Counts are
/// computed by the read repository so the client does not fetch four resource
/// collections to render a single row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamDirectoryItem {
    pub team: Team,
    pub role: Role,
    pub member_count: u64,
    pub active_incident_count: u64,
    pub active_release_count: u64,
    pub blocked_release_count: u64,
    pub image_updated_at: Option<DateTime<Utc>>,
}

pub const MAX_TEAM_IMAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamImage {
    pub media_type: String,
    pub content: Vec<u8>,
    pub updated_at: DateTime<Utc>,
}

impl TeamImage {
    pub fn new(media_type: impl Into<String>, content: Vec<u8>) -> Result<Self, DomainError> {
        let media_type = media_type.into();
        let signature_matches = match media_type.as_str() {
            "image/png" => content.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => content.starts_with(&[0xff, 0xd8, 0xff]),
            "image/webp" => {
                content.len() >= 12 && &content[..4] == b"RIFF" && &content[8..12] == b"WEBP"
            }
            _ => false,
        };
        if content.is_empty() || content.len() > MAX_TEAM_IMAGE_BYTES || !signature_matches {
            return Err(DomainError::InvalidTeamImage);
        }
        Ok(Self {
            media_type,
            content,
            updated_at: Utc::now(),
        })
    }
}

/// A single role assignment to apply. A manager transfer yields exactly two of
/// these, applied atomically by the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleChange {
    pub user_id: Uuid,
    pub new_role: Role,
}

/// The two simultaneous role changes that uphold the single-Manager invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerTransfer {
    /// The outgoing manager, downgraded to Responder.
    pub demoted: RoleChange,
    /// The incoming manager, promoted from their previous role.
    pub promoted: RoleChange,
}

/// Pure single-Manager invariant. A team must always have **exactly one**
/// Manager: handing over management is never "add a Manager" but an atomic
/// swap that demotes the current one to Responder while promoting the next.
///
/// `requester_role` is the role the caller actually holds (resolved from the
/// repository), so this also enforces RBAC: only a Manager may transfer.
pub fn plan_manager_transfer(
    requester_role: Role,
    requester_id: Uuid,
    new_manager_id: Uuid,
) -> Result<ManagerTransfer, DomainError> {
    if requester_role != Role::Manager {
        return Err(DomainError::NotManager);
    }
    if requester_id == new_manager_id {
        return Err(DomainError::AlreadyManager);
    }
    Ok(ManagerTransfer {
        demoted: RoleChange {
            user_id: requester_id,
            new_role: Role::Responder,
        },
        promoted: RoleChange {
            user_id: new_manager_id,
            new_role: Role::Manager,
        },
    })
}

/// Pure validation for an Observer↔Responder role change. This endpoint never
/// touches the Manager role: promotion to Manager is a transfer, and demoting
/// the sitting Manager here would break the single-Manager invariant. Both are
/// rejected so the only authority over the Manager seat stays
/// `plan_manager_transfer`.
pub fn validate_member_role_change(
    requester_role: Role,
    target_current_role: Role,
    new_role: Role,
) -> Result<(), DomainError> {
    if requester_role != Role::Manager {
        return Err(DomainError::NotManager);
    }
    if new_role == Role::Manager {
        return Err(DomainError::InvalidRole);
    }
    if target_current_role == Role::Manager {
        return Err(DomainError::CannotChangeManagerRole);
    }
    Ok(())
}

/// How long a moderation ban keeps a user out of a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BanKind {
    /// Blocks (re)joining until `expires_at`.
    Temporary { expires_at: DateTime<Utc> },
    /// Blocks (re)joining with no end.
    Permanent,
}

/// A moderation ban: a user barred from a team by its Manager. At most one ban
/// row exists per (team, user); re-banning replaces it. A ban is independent of
/// membership — it persists after the membership row is removed, which is what
/// blocks a later rejoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamBan {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub kind: BanKind,
    pub reason: Option<String>,
    /// The moderator who issued the ban, or `None` once that account is deleted
    /// (the FK is `ON DELETE SET NULL`, so the ban outlives its issuer).
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A ban enriched with the identities needed by the moderation UI. The banned
/// user always exists while the row exists; the moderator may have been
/// deleted, hence the optional identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamBanView {
    pub ban: TeamBan,
    pub user_email: String,
    pub moderator_email: Option<String>,
}

impl TeamBan {
    /// A temporary ban. Rejected if `expires_at` is not in the future (a ban
    /// that expired the moment it was created would be meaningless).
    pub fn temporary_at(
        team_id: Uuid,
        user_id: Uuid,
        created_by: Uuid,
        expires_at: DateTime<Utc>,
        reason: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if expires_at <= now {
            return Err(DomainError::InvalidBanExpiry);
        }
        Ok(Self {
            team_id,
            user_id,
            kind: BanKind::Temporary { expires_at },
            reason,
            created_by: Some(created_by),
            created_at: now,
        })
    }

    pub fn temporary(
        team_id: Uuid,
        user_id: Uuid,
        created_by: Uuid,
        expires_at: DateTime<Utc>,
        reason: Option<String>,
    ) -> Result<Self, DomainError> {
        Self::temporary_at(team_id, user_id, created_by, expires_at, reason, Utc::now())
    }

    /// A permanent ban (no expiry).
    pub fn permanent(
        team_id: Uuid,
        user_id: Uuid,
        created_by: Uuid,
        reason: Option<String>,
    ) -> Self {
        Self {
            team_id,
            user_id,
            kind: BanKind::Permanent,
            reason,
            created_by: Some(created_by),
            created_at: Utc::now(),
        }
    }

    /// True while the ban still blocks joining: permanent bans always, temporary
    /// bans only until their expiry.
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        match self.kind {
            BanKind::Permanent => true,
            BanKind::Temporary { expires_at } => expires_at > now,
        }
    }

    /// Expiry instant for a temporary ban; `None` for a permanent one (the form
    /// persisted in `team_bans.expires_at`).
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        match self.kind {
            BanKind::Temporary { expires_at } => Some(expires_at),
            BanKind::Permanent => None,
        }
    }
}

/// Pure validation for a Manager moderating (kicking or banning) a target.
/// `target_role` is `None` when the target is not a current member (allowed only
/// for a pre-emptive ban). The single-Manager invariant means the only Manager
/// is the requester, so self- and Manager-targets are both barred.
pub fn validate_member_moderation(
    requester_id: Uuid,
    target_id: Uuid,
    target_role: Option<Role>,
) -> Result<(), DomainError> {
    if requester_id == target_id {
        return Err(DomainError::CannotModerateSelf);
    }
    if target_role == Some(Role::Manager) {
        return Err(DomainError::CannotModerateManager);
    }
    Ok(())
}

// --- TESTS ---

#[cfg(test)]
mod tests;
