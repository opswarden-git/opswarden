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

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Role::Observer => "observer",
            Role::Responder => "responder",
            Role::Manager => "manager",
        };
        f.write_str(value)
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
        })
    }
}

/// The association of a user with a team under a given role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: Role,
}

/// Read projection of a team member enriched with the user's email, for the
/// roster view. Joins `team_members` with `users`, so it carries identity the
/// bare `TeamMember` association does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMemberView {
    pub user_id: Uuid,
    pub email: String,
    pub role: Role,
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

impl TeamBan {
    /// A temporary ban. Rejected if `expires_at` is not in the future (a ban
    /// that expired the moment it was created would be meaningless).
    pub fn temporary(
        team_id: Uuid,
        user_id: Uuid,
        created_by: Uuid,
        expires_at: DateTime<Utc>,
        reason: Option<String>,
    ) -> Result<Self, DomainError> {
        if expires_at <= Utc::now() {
            return Err(DomainError::InvalidBanExpiry);
        }
        Ok(Self {
            team_id,
            user_id,
            kind: BanKind::Temporary { expires_at },
            reason,
            created_by: Some(created_by),
            created_at: Utc::now(),
        })
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
mod tests {
    use super::*;

    #[test]
    fn role_hierarchy_grants_lower_privileges() {
        assert!(Role::Manager.can_act_as(Role::Responder));
        assert!(Role::Manager.can_act_as(Role::Observer));
        assert!(Role::Responder.can_act_as(Role::Observer));
        assert!(Role::Observer.can_act_as(Role::Observer));
    }

    #[test]
    fn role_hierarchy_denies_higher_privileges() {
        assert!(!Role::Observer.can_act_as(Role::Responder));
        assert!(!Role::Observer.can_act_as(Role::Manager));
        assert!(!Role::Responder.can_act_as(Role::Manager));
    }

    #[test]
    fn invitation_code_is_prefixed_and_well_formed() {
        let code = InvitationCode::generate();
        let value = code.as_str();

        assert!(value.starts_with("OPS-"));
        assert_eq!(value.len(), 4 + CODE_LEN);
        assert!(value[4..].bytes().all(|b| CODE_ALPHABET.contains(&b)));
    }

    #[test]
    fn team_creation_generates_id_and_code() {
        let team = Team::new("SRE Core").unwrap();

        assert_eq!(team.name, "SRE Core");
        assert_eq!(team.id.to_string().len(), 36);
        assert!(team.invitation_code.as_str().starts_with("OPS-"));
    }

    #[test]
    fn team_creation_rejects_blank_name() {
        let result = Team::new("   ");

        assert_eq!(result.unwrap_err(), DomainError::InvalidTeamName);
    }

    #[test]
    fn transfer_demotes_old_manager_and_promotes_new() {
        let old = Uuid::new_v4();
        let new = Uuid::new_v4();

        let transfer = plan_manager_transfer(Role::Manager, old, new).unwrap();

        assert_eq!(
            transfer.demoted,
            RoleChange {
                user_id: old,
                new_role: Role::Responder
            }
        );
        assert_eq!(
            transfer.promoted,
            RoleChange {
                user_id: new,
                new_role: Role::Manager
            }
        );
    }

    #[test]
    fn transfer_is_refused_to_non_manager() {
        let requester = Uuid::new_v4();
        let target = Uuid::new_v4();

        let result = plan_manager_transfer(Role::Responder, requester, target);

        assert_eq!(result.unwrap_err(), DomainError::NotManager);
    }

    #[test]
    fn transfer_to_self_is_rejected() {
        let manager = Uuid::new_v4();

        let result = plan_manager_transfer(Role::Manager, manager, manager);

        assert_eq!(result.unwrap_err(), DomainError::AlreadyManager);
    }

    #[test]
    fn manager_may_promote_and_demote_between_observer_and_responder() {
        assert!(
            validate_member_role_change(Role::Manager, Role::Observer, Role::Responder).is_ok()
        );
        assert!(
            validate_member_role_change(Role::Manager, Role::Responder, Role::Observer).is_ok()
        );
    }

    #[test]
    fn non_manager_cannot_change_roles() {
        assert_eq!(
            validate_member_role_change(Role::Responder, Role::Observer, Role::Responder)
                .unwrap_err(),
            DomainError::NotManager
        );
    }

    #[test]
    fn promotion_to_manager_is_not_a_role_change() {
        assert_eq!(
            validate_member_role_change(Role::Manager, Role::Responder, Role::Manager).unwrap_err(),
            DomainError::InvalidRole
        );
    }

    #[test]
    fn the_sitting_manager_role_cannot_be_changed_here() {
        assert_eq!(
            validate_member_role_change(Role::Manager, Role::Manager, Role::Responder).unwrap_err(),
            DomainError::CannotChangeManagerRole
        );
    }

    #[test]
    fn a_permanent_ban_is_always_active() {
        let ban = TeamBan::permanent(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), None);
        assert!(ban.is_active(Utc::now()));
        assert_eq!(ban.expires_at(), None);
    }

    #[test]
    fn a_temporary_ban_is_active_before_expiry_and_inactive_after() {
        let expires = Utc::now() + chrono::Duration::hours(1);
        let ban = TeamBan::temporary(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            expires,
            None,
        )
        .unwrap();
        assert!(ban.is_active(Utc::now()));
        assert!(!ban.is_active(expires + chrono::Duration::seconds(1)));
        assert_eq!(ban.expires_at(), Some(expires));
    }

    #[test]
    fn a_temporary_ban_in_the_past_is_rejected() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let result = TeamBan::temporary(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), past, None);
        assert_eq!(result.unwrap_err(), DomainError::InvalidBanExpiry);
    }

    #[test]
    fn moderation_bars_self_and_manager_targets() {
        let manager = Uuid::new_v4();
        let member = Uuid::new_v4();

        assert!(validate_member_moderation(manager, member, Some(Role::Observer)).is_ok());
        assert!(validate_member_moderation(manager, member, Some(Role::Responder)).is_ok());
        // Pre-emptive ban of a non-member is allowed.
        assert!(validate_member_moderation(manager, member, None).is_ok());

        assert_eq!(
            validate_member_moderation(manager, manager, Some(Role::Manager)).unwrap_err(),
            DomainError::CannotModerateSelf
        );
        assert_eq!(
            validate_member_moderation(manager, member, Some(Role::Manager)).unwrap_err(),
            DomainError::CannotModerateManager
        );
    }
}
