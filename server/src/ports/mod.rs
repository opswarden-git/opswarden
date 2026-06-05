//! Hexagonal ports (traits).
//!
//! Adapters implement these; use-cases depend on them. Repositories and the
//! event bus arrive from S1. Example below keeps use-cases deterministic.

/// Abstracts the system clock so use-cases can be tested deterministically.
pub trait Clock: Send + Sync {
    /// Current Unix time in seconds.
    fn now_unix(&self) -> u64;
}
