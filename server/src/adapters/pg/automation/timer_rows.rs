use std::str::FromStr;

use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::automation_timer::{TimerSchedule, DAILY_AT_KIND, EVERY_MINUTES_KIND};
use crate::domain::error::DomainError;

#[derive(FromRow)]
pub(super) struct DueScheduleRow {
    pub(super) rule_id: Uuid,
    pub(super) team_id: Uuid,
    pub(super) connection_id: Uuid,
    schedule_kind: String,
    timezone: String,
    pub(super) local_time: Option<NaiveTime>,
    pub(super) interval_minutes: Option<i32>,
    pub(super) scheduled_for: DateTime<Utc>,
    pub(super) rule_updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
pub(super) struct UnstartedClaimRow {
    pub(super) rule_id: Uuid,
    pub(super) team_id: Uuid,
    pub(super) connection_id: Uuid,
    pub(super) delivery_id: Uuid,
    pub(super) schedule_kind: String,
    pub(super) timezone: String,
    pub(super) local_time: Option<NaiveTime>,
    pub(super) interval_minutes: Option<i32>,
    pub(super) scheduled_for: DateTime<Utc>,
    pub(super) claimed_at: DateTime<Utc>,
    pub(super) rule_updated_at: DateTime<Utc>,
}

pub(super) fn stored_schedule(
    kind: &str,
    timezone: &str,
    local_time: Option<NaiveTime>,
    interval_minutes: Option<i32>,
) -> Result<TimerSchedule, DomainError> {
    let timezone = Tz::from_str(timezone).map_err(|_| DomainError::Storage)?;
    match kind {
        DAILY_AT_KIND => Ok(TimerSchedule::DailyAt {
            time: local_time.ok_or(DomainError::Storage)?,
            timezone,
        }),
        EVERY_MINUTES_KIND => Ok(TimerSchedule::EveryMinutes {
            minutes: interval_minutes
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(DomainError::Storage)?,
            timezone,
        }),
        _ => Err(DomainError::Storage),
    }
}

impl DueScheduleRow {
    pub(super) fn schedule(&self) -> Result<TimerSchedule, DomainError> {
        stored_schedule(
            &self.schedule_kind,
            &self.timezone,
            self.local_time,
            self.interval_minutes,
        )
    }
}
