mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::incident::{Incident, Severity};
use opswarden_server::domain::team::Role;
use opswarden_server::domain::timeline::TimelineEntry;
use opswarden_server::domain::user::{Email, User};
use tower::ServiceExt;
use uuid::Uuid;

include!("incidents/lifecycle.rs");
include!("incidents/listing.rs");
include!("incidents/collaboration.rs");
