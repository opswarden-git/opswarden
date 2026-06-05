//! OpsWarden server -- modular hexagonal monolith.
//!
//! Dependency rule: everything points inward toward `domain`.
//! `handlers` (Axum) -> `app` (use-cases) -> `ports` (traits) -> `domain` (pure).
//! `adapters` implement `ports` (Postgres, WS broadcaster, crypto vault).
#![forbid(unsafe_code)]

pub mod adapters;
pub mod app;
pub mod config;
pub mod domain;
pub mod handlers;
pub mod ports;

use axum::{routing::get, Router};

use crate::config::Config;

/// Build the HTTP application without any I/O side effect, so it can be tested
/// without binding a socket.
pub fn build_app(config: Config) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/about.json", get(handlers::about))
        .with_state(config)
}
