use chrono::{DateTime, Utc};

use crate::domain::team::{BanKind, Role};

pub(super) fn role_from_str(value: &str) -> Result<Role, crate::domain::error::DomainError> {
    Role::try_from(value).map_err(|_| crate::domain::error::DomainError::Storage)
}

/// Map the nullable `team_bans.expires_at` column to a `BanKind`
/// (NULL = permanent, a timestamp = temporary).
pub(super) fn ban_kind(expires_at: Option<DateTime<Utc>>) -> BanKind {
    match expires_at {
        Some(expires_at) => BanKind::Temporary { expires_at },
        None => BanKind::Permanent,
    }
}
