//! Pure business models and invariants.
//!
//! No dependency on Axum, SQLx, or the network. Populated from S1 onward:
//! `User`, `Team`, `Membership`, `Incident`, `Release`, `Rule`, plus the state
//! machines (incident: open -> acknowledged -> escalated -> resolved).
