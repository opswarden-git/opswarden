use serde::{Deserialize, Serialize};

use super::team::Role;

/// Product actions derived from one team membership.
///
/// The server remains the security authority. The web client mirrors this
/// contract only to avoid rendering actions that the server will reject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub can_send_private_message: bool,
}

pub fn derive_capabilities(role: Role) -> TeamCapabilities {
    let responder_or_manager = role.can_act_as(Role::Responder);
    let manager = role == Role::Manager;

    TeamCapabilities {
        can_create_incident: manager,
        can_transition_incident: responder_or_manager,
        can_assign_incident: manager,
        can_delete_incident: manager,
        can_write_timeline: responder_or_manager,
        can_signal_typing: responder_or_manager,
        can_react_timeline: true,
        can_create_release: manager,
        can_progress_release: responder_or_manager,
        can_link_release_incident: responder_or_manager,
        can_cancel_release: manager,
        can_manage_members: manager,
        can_manage_automations: manager,
        can_view_invitation_code: manager,
        can_leave_team: !manager,
        can_delete_team: manager,
        can_send_private_message: true,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn rust_capabilities_match_the_shared_contract() {
        let contract: HashMap<String, TeamCapabilities> =
            serde_json::from_str(include_str!("../../../contracts/role-capabilities.json"))
                .expect("valid role capability contract");

        for (name, role) in [
            ("observer", Role::Observer),
            ("responder", Role::Responder),
            ("manager", Role::Manager),
        ] {
            assert_eq!(contract.get(name), Some(&derive_capabilities(role)));
        }
    }
}
