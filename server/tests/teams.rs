mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::Engine as _;
use common::test_context;
use opswarden_server::domain::team::{Role, Team, TeamBan};
use opswarden_server::ports::TeamRepo;
use tower::ServiceExt;
use uuid::Uuid;

include!("teams/core.rs");
include!("teams/roles.rs");
include!("teams/membership.rs");
include!("teams/image.rs");
