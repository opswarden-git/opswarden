//! In-process token-bucket rate limiter, keyed by an opaque caller identity.
//!
//! It exists to bound credential stuffing against `/api/auth/*`, which was the
//! one externally reachable surface with no ceiling of its own. Everything else
//! is already bounded: bodies are capped per route, the outbound HTTP REAction
//! pins its resolved address, and every Team route resolves a role.
//!
//! The key is a string rather than an address because an address is not always
//! the caller. Compose puts the Next client in front of the server, and its
//! `rewrites()` forward no `X-Forwarded-For`, so every browser resolves to the
//! proxy container and shares one budget — one visitor could lock out the whole
//! deployment. Sign-in is therefore keyed by the account being attacked, which
//! no proxy topology can blur, with a loose per-address ceiling behind it.
//!
//! Deliberately dependency-free rather than pulling `tower_governor`: the crate
//! tree here is curated and `tooling/deny.toml` gates licences, so ~100 lines of
//! owned code beats four transitive dependencies for one middleware.
//!
//! State lives in this process, so a deployment running N replicas allows N
//! times the configured budget. That is accepted: the goal is to turn an
//! unbounded guessing loop into a bounded one, not to meter API quota.

use chrono::{DateTime, Duration, Utc};
use std::{collections::HashMap, sync::Mutex};

/// Entries idle for longer than this multiple of the window are dropped, so a
/// scan of attacker-generated keys cannot grow the map without bound.
const PRUNE_WINDOW_MULTIPLIER: i32 = 2;

/// Prune only once the map is worth scanning; keeps the common path O(1).
const PRUNE_THRESHOLD: usize = 1024;

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    last_refill: DateTime<Utc>,
}

/// Outcome of a rate-limit check.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// Denied; the caller should retry after this many seconds.
    Deny {
        retry_after_seconds: u64,
    },
}

pub struct RateLimiter {
    capacity: f64,
    window: Duration,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    /// `capacity` requests per `window_seconds`, refilled continuously so a
    /// caller regains one slot every `window / capacity`.
    pub fn new(capacity: u32, window_seconds: u64) -> Self {
        Self {
            capacity: f64::from(capacity.max(1)),
            window: Duration::seconds(window_seconds.max(1) as i64),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Consume one token for `key`. `now` is injected so the behaviour is
    /// testable without sleeping.
    pub fn check(&self, key: &str, now: DateTime<Utc>) -> Decision {
        self.take(key, now, true)
    }

    /// Whether `key` has budget left, without spending any.
    ///
    /// Pairs with [`record_failure`]: a limiter that meters *attempts* also
    /// meters the legitimate ones, and a successful sign-in is evidence that
    /// the caller is not guessing. Counting only failures keeps a busy team —
    /// or a test suite — out of a bucket meant for an attacker.
    pub fn peek(&self, key: &str, now: DateTime<Utc>) -> Decision {
        self.take(key, now, false)
    }

    /// Spend one token for `key`, ignoring whether any remained.
    pub fn record_failure(&self, key: &str, now: DateTime<Utc>) {
        let _ = self.take(key, now, true);
    }

    fn take(&self, key: &str, now: DateTime<Utc>, spend: bool) -> Decision {
        let window_seconds = self.window.num_seconds().max(1) as f64;
        let refill_per_second = self.capacity / window_seconds;

        // A poisoned lock must not take authentication down; recovering keeps
        // the limiter conservative (the stored counts survive).
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if buckets.len() >= PRUNE_THRESHOLD {
            let cutoff = now - self.window * PRUNE_WINDOW_MULTIPLIER;
            buckets.retain(|_, bucket| bucket.last_refill > cutoff);
        }

        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        let elapsed = (now - bucket.last_refill).num_milliseconds().max(0) as f64 / 1000.0;
        bucket.tokens = (bucket.tokens + elapsed * refill_per_second).min(self.capacity);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            if spend {
                bucket.tokens -= 1.0;
            }
            return Decision::Allow;
        }

        let missing = 1.0 - bucket.tokens;
        let seconds = (missing / refill_per_second).ceil().max(1.0);
        Decision::Deny {
            retry_after_seconds: seconds as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> String {
        format!("203.0.113.{last}")
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000 + seconds, 0).unwrap()
    }

    #[test]
    fn allows_up_to_capacity_then_denies() {
        let limiter = RateLimiter::new(5, 60);
        for _ in 0..5 {
            assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        }
        assert!(matches!(
            limiter.check(&ip(1), at(0)),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn each_address_gets_its_own_budget() {
        let limiter = RateLimiter::new(2, 60);
        assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        assert!(matches!(
            limiter.check(&ip(1), at(0)),
            Decision::Deny { .. }
        ));
        // A different caller is untouched by the first one's exhaustion.
        assert_eq!(limiter.check(&ip(2), at(0)), Decision::Allow);
    }

    #[test]
    fn refills_over_time() {
        let limiter = RateLimiter::new(6, 60);
        for _ in 0..6 {
            assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        }
        assert!(matches!(
            limiter.check(&ip(1), at(0)),
            Decision::Deny { .. }
        ));
        // 6 per 60s == one slot every 10s.
        assert_eq!(limiter.check(&ip(1), at(10)), Decision::Allow);
        assert!(matches!(
            limiter.check(&ip(1), at(10)),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn a_full_window_restores_the_whole_budget() {
        let limiter = RateLimiter::new(3, 60);
        for _ in 0..3 {
            assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        }
        for _ in 0..3 {
            assert_eq!(limiter.check(&ip(1), at(60)), Decision::Allow);
        }
    }

    #[test]
    fn retry_after_is_never_zero_and_matches_the_refill_rate() {
        let limiter = RateLimiter::new(6, 60);
        for _ in 0..6 {
            limiter.check(&ip(1), at(0));
        }
        match limiter.check(&ip(1), at(0)) {
            Decision::Deny {
                retry_after_seconds,
            } => assert_eq!(retry_after_seconds, 10),
            Decision::Allow => panic!("expected the budget to be exhausted"),
        }
    }

    #[test]
    fn tokens_never_exceed_capacity_after_a_long_idle_period() {
        let limiter = RateLimiter::new(2, 60);
        assert_eq!(limiter.check(&ip(1), at(0)), Decision::Allow);
        // Idle for an hour: the bucket refills to capacity, not beyond.
        assert_eq!(limiter.check(&ip(1), at(3600)), Decision::Allow);
        assert_eq!(limiter.check(&ip(1), at(3600)), Decision::Allow);
        assert!(matches!(
            limiter.check(&ip(1), at(3600)),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn idle_entries_are_pruned_once_the_map_is_large() {
        let limiter = RateLimiter::new(1, 1);
        for octet in 0..=255u8 {
            for third in 0..=4u8 {
                limiter.check(&format!("198.51.{third}.{octet}"), at(0));
            }
        }
        assert!(limiter.buckets.lock().unwrap().len() >= PRUNE_THRESHOLD);
        // A later check past the prune horizon collapses the idle keys.
        limiter.check(&ip(9), at(600));
        assert!(limiter.buckets.lock().unwrap().len() < PRUNE_THRESHOLD);
    }
}
