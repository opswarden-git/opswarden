// --- server/src/lib.rs ---

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

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/about.json", get(handlers::about))
        .with_state(state)
}