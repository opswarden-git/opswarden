use chrono::{DateTime, Utc};

use crate::domain::team::{BanKind, Role};

/// `Role` as stored in the `team_members.role` text column (kept out of the
/// domain so `Role` stays free of persistence concerns).
pub(super) fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Observer => "observer",
        Role::Responder => "responder",
        Role::Manager => "manager",
    }
}

/// Inverse of `role_to_str`. The DB `check` constraint guarantees a valid value;
/// anything unexpected falls back to the least-privileged role by design.
pub(super) fn role_from_str(value: &str) -> Role {
    match value {
        "manager" => Role::Manager,
        "responder" => Role::Responder,
        _ => Role::Observer,
    }
}

/// Map the nullable `team_bans.expires_at` column to a `BanKind`
/// (NULL = permanent, a timestamp = temporary).
pub(super) fn ban_kind(expires_at: Option<DateTime<Utc>>) -> BanKind {
    match expires_at {
        Some(expires_at) => BanKind::Temporary { expires_at },
        None => BanKind::Permanent,
    }
}
