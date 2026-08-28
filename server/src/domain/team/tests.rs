// --- server/src/domain/team.rs tests ---
//
// The rules of the team aggregate, exercised in their own file so the module
// itself reads as the rules rather than the rules plus their proof.

use super::*;

#[test]
fn role_hierarchy_grants_lower_privileges() {
    assert!(Role::Manager.can_act_as(Role::Responder));
    assert!(Role::Manager.can_act_as(Role::Observer));
    assert!(Role::Responder.can_act_as(Role::Observer));
    assert!(Role::Observer.can_act_as(Role::Observer));
}

#[test]
fn role_hierarchy_denies_higher_privileges() {
    assert!(!Role::Observer.can_act_as(Role::Responder));
    assert!(!Role::Observer.can_act_as(Role::Manager));
    assert!(!Role::Responder.can_act_as(Role::Manager));
}

#[test]
fn invitation_code_is_prefixed_and_well_formed() {
    let code = InvitationCode::generate();
    let value = code.as_str();

    assert!(value.starts_with("OPS-"));
    assert_eq!(value.len(), 4 + CODE_LEN);
    assert!(value[4..].bytes().all(|b| CODE_ALPHABET.contains(&b)));
}

#[test]
fn team_creation_generates_id_and_code() {
    let team = Team::new("SRE Core").unwrap();

    assert_eq!(team.name, "SRE Core");
    assert_eq!(team.id.to_string().len(), 36);
    assert!(team.invitation_code.as_str().starts_with("OPS-"));
}

#[test]
fn team_creation_rejects_blank_name() {
    let result = Team::new("   ");

    assert_eq!(result.unwrap_err(), DomainError::InvalidTeamName);
}

#[test]
fn transfer_demotes_old_manager_and_promotes_new() {
    let old = Uuid::new_v4();
    let new = Uuid::new_v4();

    let transfer = plan_manager_transfer(Role::Manager, old, new).unwrap();

    assert_eq!(
        transfer.demoted,
        RoleChange {
            user_id: old,
            new_role: Role::Responder
        }
    );
    assert_eq!(
        transfer.promoted,
        RoleChange {
            user_id: new,
            new_role: Role::Manager
        }
    );
}

#[test]
fn transfer_is_refused_to_non_manager() {
    let requester = Uuid::new_v4();
    let target = Uuid::new_v4();

    let result = plan_manager_transfer(Role::Responder, requester, target);

    assert_eq!(result.unwrap_err(), DomainError::NotManager);
}

#[test]
fn transfer_to_self_is_rejected() {
    let manager = Uuid::new_v4();

    let result = plan_manager_transfer(Role::Manager, manager, manager);

    assert_eq!(result.unwrap_err(), DomainError::AlreadyManager);
}

#[test]
fn manager_may_promote_and_demote_between_observer_and_responder() {
    assert!(validate_member_role_change(Role::Manager, Role::Observer, Role::Responder).is_ok());
    assert!(validate_member_role_change(Role::Manager, Role::Responder, Role::Observer).is_ok());
}

#[test]
fn non_manager_cannot_change_roles() {
    assert_eq!(
        validate_member_role_change(Role::Responder, Role::Observer, Role::Responder).unwrap_err(),
        DomainError::NotManager
    );
}

#[test]
fn promotion_to_manager_is_not_a_role_change() {
    assert_eq!(
        validate_member_role_change(Role::Manager, Role::Responder, Role::Manager).unwrap_err(),
        DomainError::InvalidRole
    );
}

#[test]
fn the_sitting_manager_role_cannot_be_changed_here() {
    assert_eq!(
        validate_member_role_change(Role::Manager, Role::Manager, Role::Responder).unwrap_err(),
        DomainError::CannotChangeManagerRole
    );
}

#[test]
fn a_permanent_ban_is_always_active() {
    let ban = TeamBan::permanent(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), None);
    assert!(ban.is_active(Utc::now()));
    assert_eq!(ban.expires_at(), None);
}

#[test]
fn a_temporary_ban_is_active_before_expiry_and_inactive_after() {
    let expires = Utc::now() + chrono::Duration::hours(1);
    let ban = TeamBan::temporary(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        expires,
        None,
    )
    .unwrap();
    assert!(ban.is_active(Utc::now()));
    assert!(!ban.is_active(expires + chrono::Duration::seconds(1)));
    assert_eq!(ban.expires_at(), Some(expires));
}

#[test]
fn a_temporary_ban_in_the_past_is_rejected() {
    let past = Utc::now() - chrono::Duration::hours(1);
    let result = TeamBan::temporary(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), past, None);
    assert_eq!(result.unwrap_err(), DomainError::InvalidBanExpiry);
}

#[test]
fn moderation_bars_self_and_manager_targets() {
    let manager = Uuid::new_v4();
    let member = Uuid::new_v4();

    assert!(validate_member_moderation(manager, member, Some(Role::Observer)).is_ok());
    assert!(validate_member_moderation(manager, member, Some(Role::Responder)).is_ok());
    // Pre-emptive ban of a non-member is allowed.
    assert!(validate_member_moderation(manager, member, None).is_ok());

    assert_eq!(
        validate_member_moderation(manager, manager, Some(Role::Manager)).unwrap_err(),
        DomainError::CannotModerateSelf
    );
    assert_eq!(
        validate_member_moderation(manager, member, Some(Role::Manager)).unwrap_err(),
        DomainError::CannotModerateManager
    );
}

#[test]
fn team_image_checks_size_type_and_binary_signature() {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(b"bounded-image");
    assert!(TeamImage::new("image/png", png.clone()).is_ok());
    assert_eq!(
        TeamImage::new("image/jpeg", png).unwrap_err(),
        DomainError::InvalidTeamImage
    );
    assert_eq!(
        TeamImage::new("image/svg+xml", b"<svg/>".to_vec()).unwrap_err(),
        DomainError::InvalidTeamImage
    );
    assert_eq!(
        TeamImage::new("image/png", vec![0; MAX_TEAM_IMAGE_BYTES + 1]).unwrap_err(),
        DomainError::InvalidTeamImage
    );
}
