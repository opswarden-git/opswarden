use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::team::Role;

/// Product actions derived from one team membership.
///
/// The server remains the security authority. The web client mirrors this
/// contract only to avoid rendering actions that the server will reject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct TeamCapabilities {
    pub can_create_incident: bool,
    pub can_transition_incident: bool,
    pub can_assign_incident: bool,
    pub can_delete_incident: bool,
    pub can_write_timeline: bool,
    pub can_signal_typing: bool,
    pub can_react_timeline: bool,
    pub can_create_release: bool,
    pub can_progress_release: bool,
    pub can_link_release_incident: bool,
    pub can_cancel_release: bool,
    pub can_manage_members: bool,
    pub can_manage_automations: bool,
    pub can_view_invitation_code: bool,
    pub can_leave_team: bool,
    pub can_delete_team: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleCapabilityContract {
    observer: TeamCapabilities,
    responder: TeamCapabilities,
    manager: TeamCapabilities,
}

static CAPABILITIES: LazyLock<RoleCapabilityContract> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../contracts/role-capabilities.json"))
        .expect("role-capabilities.json must match TeamCapabilities")
});

pub fn derive_capabilities(role: Role) -> TeamCapabilities {
    match role {
        Role::Observer => CAPABILITIES.observer,
        Role::Responder => CAPABILITIES.responder,
        Role::Manager => CAPABILITIES.manager,
    }
}
