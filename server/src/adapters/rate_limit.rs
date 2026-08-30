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

/// Entries idle for longer than this multiple of the window are dropped.
const PRUNE_WINDOW_MULTIPLIER: i32 = 2;

/// A process never retains more caller identities than this. At capacity, a
/// new identity is denied until an idle bucket expires; active budgets are not
/// evicted because that would let an attacker reset a victim's limit by churn.
const MAX_BUCKETS: usize = 4096;

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    last_refill: DateTime<Utc>,
}

struct BucketStore {
    buckets: HashMap<String, Bucket>,
    idle_ttl: Duration,
    max_buckets: usize,
    next_expiration: Option<DateTime<Utc>>,
}

impl BucketStore {
    fn new(idle_ttl: Duration, max_buckets: usize) -> Self {
        Self {
            buckets: HashMap::new(),
            idle_ttl,
            max_buckets: max_buckets.max(1),
            next_expiration: None,
        }
    }

    fn expire_idle(&mut self, now: DateTime<Utc>) {
        if !matches!(self.next_expiration, Some(expires_at) if expires_at <= now) {
            return;
        }
        self.buckets
            .retain(|_, bucket| bucket.last_refill + self.idle_ttl > now);
        self.next_expiration = self
            .buckets
            .values()
            .map(|bucket| bucket.last_refill + self.idle_ttl)
            .min();
    }

    fn capacity_retry_after(&self, now: DateTime<Utc>) -> u64 {
        self.next_expiration
            .map(|expires_at| (expires_at - now).num_seconds().max(1) as u64)
            .unwrap_or(1)
    }

    fn track_expiration(&mut self, key: &str) {
        if let Some(bucket) = self.buckets.get(key) {
            let expires_at = bucket.last_refill + self.idle_ttl;
            self.next_expiration = Some(
                self.next_expiration
                    .map_or(expires_at, |current| current.min(expires_at)),
            );
        }
    }
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
    buckets: Mutex<BucketStore>,
}

impl RateLimiter {
    /// `capacity` requests per `window_seconds`, refilled continuously so a
    /// caller regains one slot every `window / capacity`.
    pub fn new(capacity: u32, window_seconds: u64) -> Self {
        Self::with_max_buckets(capacity, window_seconds, MAX_BUCKETS)
    }

    fn with_max_buckets(capacity: u32, window_seconds: u64, max_buckets: usize) -> Self {
        let window = Duration::seconds(window_seconds.max(1) as i64);
        Self {
            capacity: f64::from(capacity.max(1)),
            window,
            buckets: Mutex::new(BucketStore::new(
                window * PRUNE_WINDOW_MULTIPLIER,
                max_buckets,
            )),
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
        let mut store = self
            .buckets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        store.expire_idle(now);

        if !store.buckets.contains_key(key) {
            // Peeking at a previously unseen account must not let arbitrary
            // login identifiers consume the bounded store. Its first failed
            // attempt records the bucket through `record_failure` instead.
            if !spend {
                return Decision::Allow;
            }
            if store.buckets.len() >= store.max_buckets {
                return Decision::Deny {
                    retry_after_seconds: store.capacity_retry_after(now),
                };
            }
            store.buckets.insert(
                key.to_string(),
                Bucket {
                    tokens: self.capacity,
                    last_refill: now,
                },
            );
        }

        let bucket = store
            .buckets
            .get_mut(key)
            .expect("a rate-limit bucket was inserted or already present");

        let elapsed = (now - bucket.last_refill).num_milliseconds().max(0) as f64 / 1000.0;
        bucket.tokens = (bucket.tokens + elapsed * refill_per_second).min(self.capacity);
        bucket.last_refill = now;

        let decision = if bucket.tokens >= 1.0 {
            if spend {
                bucket.tokens -= 1.0;
            }
            Decision::Allow
        } else {
            let missing = 1.0 - bucket.tokens;
            let seconds = (missing / refill_per_second).ceil().max(1.0);
            Decision::Deny {
                retry_after_seconds: seconds as u64,
            }
        };
        store.track_expiration(key);
        decision
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
    fn unique_keys_never_grow_the_store_past_its_capacity() {
        let limiter = RateLimiter::with_max_buckets(1, 60, 3);
        for key in ["one", "two", "three"] {
            assert_eq!(limiter.check(key, at(0)), Decision::Allow);
        }

        assert_eq!(
            limiter.check("four", at(0)),
            Decision::Deny {
                retry_after_seconds: 120,
            }
        );
        assert_eq!(limiter.buckets.lock().unwrap().buckets.len(), 3);
    }

    #[test]
    fn capacity_returns_exactly_when_idle_buckets_expire() {
        let limiter = RateLimiter::with_max_buckets(1, 10, 1);
        assert_eq!(limiter.check("first", at(0)), Decision::Allow);
        assert!(matches!(
            limiter.check("second", at(19)),
            Decision::Deny {
                retry_after_seconds: 1
            }
        ));

        assert_eq!(limiter.check("second", at(20)), Decision::Allow);
        let store = limiter.buckets.lock().unwrap();
        assert_eq!(store.buckets.len(), 1);
        assert!(store.buckets.contains_key("second"));
    }

    #[test]
    fn touching_a_bucket_extends_its_idle_lifetime() {
        let limiter = RateLimiter::with_max_buckets(2, 10, 1);
        assert_eq!(limiter.check("active", at(0)), Decision::Allow);
        assert_eq!(limiter.check("active", at(19)), Decision::Allow);

        assert_eq!(
            limiter.check("new", at(20)),
            Decision::Deny {
                retry_after_seconds: 19,
            }
        );
        assert!(limiter
            .buckets
            .lock()
            .unwrap()
            .buckets
            .contains_key("active"));
    }

    #[test]
    fn peeking_at_unknown_accounts_does_not_allocate_buckets() {
        let limiter = RateLimiter::with_max_buckets(1, 60, 2);
        for key in ["one@example.com", "two@example.com", "three@example.com"] {
            assert_eq!(limiter.peek(key, at(0)), Decision::Allow);
        }
        assert!(limiter.buckets.lock().unwrap().buckets.is_empty());

        limiter.record_failure("one@example.com", at(0));
        assert_eq!(limiter.buckets.lock().unwrap().buckets.len(), 1);
    }
}
