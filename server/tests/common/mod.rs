use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opswarden_server::adapters::crypto::hmac::HmacSha256Verifier;
use opswarden_server::adapters::ws::WsHub;
use opswarden_server::domain::automation_config::{
    AutomationRule, AutomationRun, CredentialKind, ServiceConnection, WebhookDelivery,
};
use opswarden_server::domain::error::DomainError;
use opswarden_server::domain::incident::{Incident, IncidentStatus};
use opswarden_server::domain::incident_event::IncidentEvent;
use opswarden_server::domain::private_message::PrivateMessage;
use opswarden_server::domain::release::{Release, ReleaseState};
use opswarden_server::domain::team::{
    Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamMemberView,
};
use opswarden_server::domain::timeline::{ReactionRecord, TimelineEntry};
use opswarden_server::domain::user::{Locale, User};
use opswarden_server::ports::{
    AutomationRuleRepo, AutomationRunRepo, Clock, ConnectionCredentialVault, EmailMessage,
    EmailSender, GifResult, GifSearch, IncidentRepo, Notifier, OAuthClient, OAuthProfile,
    PasswordHasher, PrivateMessageRepo, ReleaseRepo, ServiceConnectionRepo, ServiceOAuthClient,
    ServiceOAuthTokens, SmtpConfig, TeamRepo, TimelineRepo, TokenClaims, TokenRevocationRepo,
    TokenService, UserRepo, WebhookDeliveryRepo,
};
use opswarden_server::{build_app, config::Config, AppState};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestContext {
    pub app: axum::Router,
    pub users: Arc<DummyUserRepo>,
    pub teams: Arc<DummyTeamRepo>,
    pub incidents: Arc<DummyIncidentRepo>,
    pub timeline: Arc<DummyTimelineRepo>,
    pub private_messages: Arc<DummyPrivateMessageRepo>,
    pub releases: Arc<DummyReleaseRepo>,
    pub revoked_tokens: Arc<DummyTokenRevocationRepo>,
    pub events: Arc<WsHub>,
    pub service_connections: Arc<DummyServiceConnectionRepo>,
    pub connection_credentials: Arc<DummyConnectionCredentialVault>,
    pub service_oauth: Arc<DummyServiceOAuthClient>,
    pub automation_rules: Arc<DummyAutomationRuleRepo>,
    pub webhook_deliveries: Arc<DummyWebhookDeliveryRepo>,
    pub automation_runs: Arc<DummyAutomationRunRepo>,
    pub notifier: Arc<DummyNotifier>,
    pub email_sender: Arc<DummyEmailSender>,
}

include!("automation.rs");
include!("auth.rs");
include!("teams_incidents.rs");
include!("messaging.rs");
include!("app.rs");
