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


use std::sync::Arc;
use crate::ports::{Clock, PasswordHasher, TokenService, UserRepo};

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<dyn UserRepo + Send + Sync>,
    pub hasher: Arc<dyn PasswordHasher + Send + Sync>,
    pub tokens: Arc<dyn TokenService + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub config: config::Config,
}

/// Build the HTTP application without any I/O side effect, so it can be tested
/// without binding a socket.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/about.json", get(handlers::about))
        .with_state(state)
}