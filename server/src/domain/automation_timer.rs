use std::str::FromStr;

use chrono::{DateTime, Duration, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::{automation::ExternalEvent, error::DomainError};

pub const TIMER_SERVICE: &str = "timer";
pub const DAILY_AT_KIND: &str = "daily_at";
pub const EVERY_MINUTES_KIND: &str = "every_minutes";
pub const MIN_INTERVAL_MINUTES: u16 = 5;
pub const MAX_INTERVAL_MINUTES: u16 = 1_440;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerSchedule {
    DailyAt { time: NaiveTime, timezone: Tz },
    EveryMinutes { minutes: u16, timezone: Tz },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerOccurrence {
    pub rule_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub event: ExternalEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerScheduleProjection {
    pub rule_id: Uuid,
    pub connection_id: Uuid,
    pub schedule: TimerSchedule,
    pub next_run_at: DateTime<Utc>,
    pub rule_updated_at: DateTime<Utc>,
    pub last_claimed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedTimerOccurrence {
    pub rule_id: Uuid,
    pub team_id: Uuid,
    pub connection_id: Uuid,
    pub delivery_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub claimed_at: DateTime<Utc>,
    pub rule_updated_at: DateTime<Utc>,
    pub schedule: TimerSchedule,
}

impl ClaimedTimerOccurrence {
    pub fn provider_delivery_id(&self) -> String {
        format!("timer:{}:{}", self.rule_id, self.scheduled_for.timestamp())
    }
}

impl TimerSchedule {
    pub fn from_config(kind: &str, config: &Value) -> Result<Self, DomainError> {
        let values = config
            .as_object()
            .ok_or(DomainError::InvalidTimerSchedule)?;
        let timezone = parse_timezone(values)?;
        match kind {
            DAILY_AT_KIND if has_exact_keys(values, &["time", "timezone"]) => {
                let raw = string_field(values, "time")?;
                if raw.len() != 5 {
                    return Err(DomainError::InvalidTimerSchedule);
                }
                let time = NaiveTime::parse_from_str(raw, "%H:%M")
                    .map_err(|_| DomainError::InvalidTimerSchedule)?;
                Ok(Self::DailyAt { time, timezone })
            }
            EVERY_MINUTES_KIND if has_exact_keys(values, &["minutes", "timezone"]) => {
                let minutes = string_field(values, "minutes")?
                    .parse::<u16>()
                    .map_err(|_| DomainError::InvalidTimerSchedule)?;
                if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&minutes) {
                    return Err(DomainError::InvalidTimerSchedule);
                }
                Ok(Self::EveryMinutes { minutes, timezone })
            }
            _ => Err(DomainError::InvalidTimerSchedule),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::DailyAt { .. } => DAILY_AT_KIND,
            Self::EveryMinutes { .. } => EVERY_MINUTES_KIND,
        }
    }

    pub fn timezone(&self) -> Tz {
        match self {
            Self::DailyAt { timezone, .. } | Self::EveryMinutes { timezone, .. } => *timezone,
        }
    }

    pub fn next_after(&self, after: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::DailyAt { time, timezone } => next_daily_after(*timezone, *time, after),
            Self::EveryMinutes { minutes, .. } => after + Duration::minutes(i64::from(*minutes)),
        }
    }

    /// Select the single bounded occurrence recovered after worker downtime.
    /// Intervals coalesce their backlog into the persisted occurrence. Daily
    /// schedules recover only the latest occurrence within the last 24 hours.
    pub fn recovery_occurrence(
        &self,
        persisted_next: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        if persisted_next > now {
            return None;
        }
        match self {
            Self::EveryMinutes { .. } => Some(persisted_next),
            Self::DailyAt { .. } if persisted_next < now - Duration::hours(24) => {
                let latest = self.next_after(now - Duration::hours(24));
                (latest <= now).then_some(latest)
            }
            Self::DailyAt { .. } => Some(persisted_next),
        }
    }

    pub fn occurrence(&self, rule_id: Uuid, scheduled_for: DateTime<Utc>) -> TimerOccurrence {
        let timezone = self.timezone();
        let local = scheduled_for.with_timezone(&timezone);
        let mut attributes = Map::new();
        attributes.insert("rule_id".into(), json!(rule_id));
        attributes.insert("scheduled_for".into(), json!(scheduled_for.to_rfc3339()));
        attributes.insert("timezone".into(), json!(timezone.to_string()));
        attributes.insert(
            "local_date".into(),
            json!(local.format("%Y-%m-%d").to_string()),
        );
        attributes.insert(
            "local_time".into(),
            json!(local.format("%H:%M").to_string()),
        );
        if let Self::EveryMinutes { minutes, .. } = self {
            attributes.insert("interval_minutes".into(), json!(minutes.to_string()));
        }
        TimerOccurrence {
            rule_id,
            scheduled_for,
            event: ExternalEvent::new(TIMER_SERVICE, self.kind()).with_attributes(attributes),
        }
    }
}

fn parse_timezone(values: &Map<String, Value>) -> Result<Tz, DomainError> {
    Tz::from_str(string_field(values, "timezone")?).map_err(|_| DomainError::InvalidTimerSchedule)
}

fn string_field<'a>(values: &'a Map<String, Value>, key: &str) -> Result<&'a str, DomainError> {
    values
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(DomainError::InvalidTimerSchedule)
}

fn has_exact_keys(values: &Map<String, Value>, expected: &[&str]) -> bool {
    values.len() == expected.len() && expected.iter().all(|key| values.contains_key(*key))
}

fn next_daily_after(timezone: Tz, time: NaiveTime, after: DateTime<Utc>) -> DateTime<Utc> {
    let local_after = after.with_timezone(&timezone);
    let mut date = local_after.date_naive();
    loop {
        let candidate = resolve_local(timezone, date.and_time(time));
        if candidate > after {
            return candidate;
        }
        date = date
            .succ_opt()
            .expect("chrono supports the next calendar date");
    }
}

fn resolve_local(timezone: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    match timezone.from_local_datetime(&local) {
        LocalResult::Single(value) => value.with_timezone(&Utc),
        LocalResult::Ambiguous(first, second) => first.min(second).with_timezone(&Utc),
        LocalResult::None => {
            // IANA DST gaps are bounded. Moving minute-by-minute gives the first
            // valid local instant after the requested wall-clock time.
            for offset in 1..=180 {
                let shifted = local + Duration::minutes(offset);
                match timezone.from_local_datetime(&shifted) {
                    LocalResult::Single(value) => return value.with_timezone(&Utc),
                    LocalResult::Ambiguous(first, second) => {
                        return first.min(second).with_timezone(&Utc)
                    }
                    LocalResult::None => {}
                }
            }
            unreachable!("an IANA timezone transition cannot create a three-hour gap")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_bounded_daily_and_interval_contracts() {
        assert!(matches!(
            TimerSchedule::from_config(
                DAILY_AT_KIND,
                &json!({"time": "09:30", "timezone": "Europe/Paris"})
            )
            .unwrap(),
            TimerSchedule::DailyAt { .. }
        ));
        assert!(TimerSchedule::from_config(
            EVERY_MINUTES_KIND,
            &json!({"minutes": "5", "timezone": "UTC"})
        )
        .is_ok());
        assert!(TimerSchedule::from_config(
            EVERY_MINUTES_KIND,
            &json!({"minutes": "1440", "timezone": "UTC"})
        )
        .is_ok());
        for invalid in ["4", "1441", "nope"] {
            assert_eq!(
                TimerSchedule::from_config(
                    EVERY_MINUTES_KIND,
                    &json!({"minutes": invalid, "timezone": "UTC"})
                )
                .unwrap_err(),
                DomainError::InvalidTimerSchedule
            );
        }
        assert!(TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "9:30", "timezone": "Europe/Paris"})
        )
        .is_err());
        assert!(TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "09:30", "timezone": "Europe/Nowhere"})
        )
        .is_err());
        assert!(TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "09:30", "timezone": "UTC", "extra": "rejected"})
        )
        .is_err());
    }

    #[test]
    fn daily_runs_strictly_after_now() {
        let schedule = TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "09:30", "timezone": "Europe/Paris"}),
        )
        .unwrap();
        let before = Utc.with_ymd_and_hms(2026, 7, 29, 7, 29, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2026, 7, 29, 7, 30, 0).unwrap();
        assert_eq!(
            schedule.next_after(before),
            Utc.with_ymd_and_hms(2026, 7, 29, 7, 30, 0).unwrap()
        );
        assert_eq!(
            schedule.next_after(after),
            Utc.with_ymd_and_hms(2026, 7, 30, 7, 30, 0).unwrap()
        );
    }

    #[test]
    fn daily_dst_gap_moves_to_first_valid_instant() {
        let schedule = TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "02:30", "timezone": "Europe/Paris"}),
        )
        .unwrap();
        let before = Utc.with_ymd_and_hms(2026, 3, 28, 23, 0, 0).unwrap();
        assert_eq!(
            schedule.next_after(before),
            Utc.with_ymd_and_hms(2026, 3, 29, 1, 0, 0).unwrap()
        );
    }

    #[test]
    fn daily_dst_overlap_uses_first_occurrence_only() {
        let schedule = TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "02:30", "timezone": "Europe/Paris"}),
        )
        .unwrap();
        let before = Utc.with_ymd_and_hms(2026, 10, 24, 23, 0, 0).unwrap();
        assert_eq!(
            schedule.next_after(before),
            Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap()
        );
    }

    #[test]
    fn interval_uses_elapsed_utc_minutes_and_event_is_bounded() {
        let schedule = TimerSchedule::from_config(
            EVERY_MINUTES_KIND,
            &json!({"minutes": "15", "timezone": "Europe/Paris"}),
        )
        .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 10, 25, 0, 55, 0).unwrap();
        assert_eq!(schedule.next_after(now), now + Duration::minutes(15));

        let rule_id = Uuid::new_v4();
        let occurrence = schedule.occurrence(rule_id, now);
        assert_eq!(occurrence.event.service, TIMER_SERVICE);
        assert_eq!(occurrence.event.kind, EVERY_MINUTES_KIND);
        assert_eq!(occurrence.event.attributes.len(), 6);
        assert_eq!(occurrence.event.attributes["rule_id"], rule_id.to_string());
        assert_eq!(occurrence.event.attributes["interval_minutes"], "15");
    }

    #[test]
    fn restart_recovery_is_bounded_for_daily_and_interval_schedules() {
        let now = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
        let daily = TimerSchedule::from_config(
            DAILY_AT_KIND,
            &json!({"time": "09:30", "timezone": "Europe/Paris"}),
        )
        .unwrap();
        let ten_days_old = Utc.with_ymd_and_hms(2026, 7, 19, 7, 30, 0).unwrap();
        assert_eq!(
            daily.recovery_occurrence(ten_days_old, now),
            Some(Utc.with_ymd_and_hms(2026, 7, 29, 7, 30, 0).unwrap())
        );

        let interval = TimerSchedule::from_config(
            EVERY_MINUTES_KIND,
            &json!({"minutes": "5", "timezone": "UTC"}),
        )
        .unwrap();
        assert_eq!(
            interval.recovery_occurrence(ten_days_old, now),
            Some(ten_days_old)
        );
    }
}
