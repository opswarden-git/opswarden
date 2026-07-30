mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_context;
use opswarden_server::domain::team::{Role, Team, TeamBan};
use opswarden_server::ports::TeamRepo;
use tower::ServiceExt;
use uuid::Uuid;

include!("teams/core.rs");
include!("teams/roles.rs");
include!("teams/membership.rs");
