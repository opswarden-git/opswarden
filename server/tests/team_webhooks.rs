mod common;

use std::collections::HashSet;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::test_context;
use opswarden_server::adapters::crypto::hmac::hmac_sha256;
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRunStatus, CredentialKind, ServiceConnection, WebhookDeliveryStatus,
};
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::incident::{Incident, IncidentStatus, Severity};
use opswarden_server::domain::release::{Release, ReleaseState};
use opswarden_server::ports::{
    AutomationRuleRepo, ConnectionCredentialVault, IncidentRepo, ReleaseRepo, ServiceConnectionRepo,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

const SECRET_A: &str = "team-a-signing-secret";
const SECRET_B: &str = "team-b-signing-secret";
const GITLAB_TOKEN: &str = "team-gitlab-token";
const GENERIC_TOKEN: &str = "team-generic-token";
const GENERIC_EVENT: &str = r#"{
    "source":"jury",
    "title":"Production deployment failed",
    "message":"Health check timed out",
    "severity":"critical",
    "external_id":"deploy-42",
    "event_url":"https://example.test/deployments/42",
    "ignored":{"token":"must-not-be-normalized"}
}"#;
const FAILED_RUN: &str = r#"{
    "repository":{"full_name":"opswarden/app"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"failure",
        "html_url":"https://github.com/opswarden/app/actions/runs/42"
    }
}"#;
const SUCCEEDED_RUN: &str = r#"{
    "repository":{"full_name":"opswarden/app"},
    "workflow_run":{
        "name":"CI",
        "head_branch":"main",
        "conclusion":"success",
        "html_url":"https://github.com/opswarden/app/actions/runs/43"
    }
}"#;
const NEW_TAG: &str = r#"{
    "ref":"refs/tags/v1.2.3",
    "created":true,
    "deleted":false,
    "after":"abcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "compare":"https://github.com/opswarden/app/compare/v1.2.3",
    "repository":{"full_name":"opswarden/app"},
    "sender":{"login":"octocat"}
}"#;
const MERGED_PULL_REQUEST: &str = r#"{
    "action":"closed",
    "number":42,
    "repository":{"full_name":"opswarden/app"},
    "pull_request":{
        "merged":true,
        "title":"Ship OpsWarden",
        "html_url":"https://github.com/opswarden/app/pull/42",
        "base":{"ref":"main"},
        "head":{"ref":"feature/opswarden"},
        "merged_by":{"login":"octocat"}
    }
}"#;
const GITLAB_FAILED_PIPELINE: &str = r#"{
    "object_kind":"pipeline",
    "object_attributes":{"status":"failed","ref":"main","name":"CI","url":"https://gitlab.com/opswarden/app/-/pipelines/42"},
    "project":{"path_with_namespace":"opswarden/app"}
}"#;
const GITLAB_SUCCEEDED_PIPELINE: &str = r#"{
    "object_kind":"pipeline",
    "object_attributes":{"status":"success","ref":"main","name":"CI","url":"https://gitlab.com/opswarden/app/-/pipelines/43"},
    "project":{"path_with_namespace":"opswarden/app"}
}"#;
const GITLAB_NEW_TAG: &str = r#"{
    "object_kind":"tag_push",
    "ref":"refs/tags/v1.2.3",
    "before":"0000000000000000000000000000000000000000",
    "after":"abcdefabcdefabcdefabcdefabcdefabcdefabcd",
    "user_username":"octocat",
    "project":{"path_with_namespace":"opswarden/app","web_url":"https://gitlab.com/opswarden/app"}
}"#;

include!("team_webhooks/helpers.rs");
include!("team_webhooks/email.rs");
include!("team_webhooks/provider_helpers.rs");
include!("team_webhooks/providers.rs");
include!("team_webhooks/notifications.rs");
include!("team_webhooks/native.rs");
