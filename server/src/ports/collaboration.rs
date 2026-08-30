use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::conversation::MessageAttachment;
use crate::domain::error::DomainError;
use crate::domain::incident::Incident;
use crate::domain::incident_event::IncidentEvent;
use crate::domain::private_message::{PrivateMessage, PrivateMessageAttachment};
use crate::domain::release::{Release, ReleaseBaseState};
use crate::domain::team::{
    Role, Team, TeamBan, TeamBanView, TeamDirectoryItem, TeamImage, TeamMemberView,
};
use crate::domain::timeline::{ReactionRecord, TimelineEntry};
use crate::domain::user::{Locale, User};

/// Position in a merged incident-activity stream: the `(created_at, id)` of the
/// oldest item a client already holds. Both activity sources order by it, so one
/// cursor walks the timeline and the system event log together.
pub type ActivityCursor = (chrono::DateTime<chrono::Utc>, Uuid);

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    async fn update_locale(&self, user_id: Uuid, locale: Locale) -> Result<(), DomainError>;
    async fn delete_account(&self, user_id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait TeamRepo: Send + Sync {
    /// Persist a team and its initial Manager in one transaction.
    async fn create_team_with_manager(
        &self,
        team: &Team,
        manager_id: Uuid,
    ) -> Result<(), DomainError>;
    /// Resolve a team from a (human-typed) invitation code.
    async fn find_by_invitation_code(&self, code: &str) -> Result<Option<Team>, DomainError>;
    /// The role a user holds in a team, or `None` if they are not a member.
    /// Lets use-cases enforce RBAC (403) without leaking membership into them.
    async fn find_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, DomainError>;
    /// Add a user to a team with the given role.
    async fn add_member(&self, team_id: Uuid, user_id: Uuid, role: Role)
        -> Result<(), DomainError>;
    /// Atomically demote `old_manager` and promote `new_manager`, upholding the
    /// single-Manager invariant in one transaction.
    async fn transfer_manager(
        &self,
        team_id: Uuid,
        old_manager: Uuid,
        new_manager: Uuid,
    ) -> Result<(), DomainError>;
    /// Every team a user belongs to. Used by the WebSocket hub to register a
    /// connection for the right broadcast scopes at connect time.
    async fn list_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
    /// Every team a user belongs to, paired with the role they hold there.
    /// Powers the dashboard's team list and lets the client gate actions by role.
    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>, DomainError>;
    /// Directory read model with operational counters for every team the user
    /// belongs to.
    async fn list_team_directory_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TeamDirectoryItem>, DomainError>;
    /// Resolve a team by its technical id for scoped detail endpoints.
    async fn find_team_by_id(&self, team_id: Uuid) -> Result<Option<Team>, DomainError>;
    /// Delete a team completely from the system.
    async fn delete_team(&self, team_id: Uuid) -> Result<(), DomainError>;
    /// Remove a user from a team.
    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    /// Revalidate moderation roles, remove the member and clear their Incident
    /// assignments in one transaction.
    async fn kick_member_and_clear_assignments(
        &self,
        team_id: Uuid,
        requester_id: Uuid,
        target_user_id: Uuid,
    ) -> Result<(), DomainError>;
    /// Count how many members a team has.
    async fn count_members(&self, team_id: Uuid) -> Result<u64, DomainError>;
    /// Every member of a team, enriched with the user's email and role. Powers
    /// the team roster view; the read is scoped to one team by the caller.
    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberView>, DomainError>;
    /// Set a member's role within a team. Used for Observer↔Responder changes;
    /// the Manager seat is upheld by `transfer_manager`, not this method.
    async fn set_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: Role,
    ) -> Result<(), DomainError>;
    /// Record (or replace) a moderation ban. Upserts on `(team_id, user_id)` so
    /// re-banning a user updates the existing row rather than duplicating it.
    async fn add_ban(&self, ban: &TeamBan) -> Result<(), DomainError>;
    /// Revalidate moderation roles, upsert the ban and atomically remove any
    /// membership and Incident assignments. Returns whether membership existed.
    async fn ban_member_and_clear_assignments(
        &self,
        ban: &TeamBan,
        requester_id: Uuid,
    ) -> Result<bool, DomainError>;
    /// The ban currently recorded for a user on a team, if any. The row may be
    /// expired; the caller decides via `TeamBan::is_active`.
    async fn find_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamBan>, DomainError>;
    /// Every ban recorded for a team, for the Manager's moderation list.
    async fn list_bans(&self, team_id: Uuid) -> Result<Vec<TeamBanView>, DomainError>;
    /// Explicitly lift a ban. Expired rows may also be removed to keep the
    /// moderation history intentional rather than silently reactivatable.
    async fn remove_ban(&self, team_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    /// Replace the single bounded identity image attached to a Team.
    async fn save_team_image(&self, team_id: Uuid, image: &TeamImage) -> Result<(), DomainError> {
        let _ = (team_id, image);
        Err(DomainError::Storage)
    }
    /// Load image bytes only for a current member of the Team.
    async fn find_team_image_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamImage>, DomainError> {
        let _ = (team_id, user_id);
        Err(DomainError::Storage)
    }
    async fn delete_team_image(&self, team_id: Uuid) -> Result<(), DomainError> {
        let _ = team_id;
        Err(DomainError::Storage)
    }
}

#[async_trait]
pub trait IncidentRepo: Send + Sync {
    async fn save_incident(&self, incident: &Incident) -> Result<(), DomainError>;
    /// Persist the initial incident and its audit event atomically.
    async fn save_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError>;
    async fn find_incident_by_id(&self, incident_id: Uuid)
        -> Result<Option<Incident>, DomainError>;
    /// Persist a mutation and the event describing it in one transaction.
    async fn update_incident_with_event(
        &self,
        incident: &Incident,
        event: &IncidentEvent,
        expected_updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError>;
    /// Newest first. `before` is a `(created_at, id)` keyset cursor: only rows
    /// strictly older than it are returned, so a war room can walk back through
    /// a long incident without the page boundary shifting under it.
    async fn list_events_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<IncidentEvent>, DomainError>;
    async fn list_incidents_for_team(&self, team_id: Uuid) -> Result<Vec<Incident>, DomainError>;
    /// Incident channels containing activity newer than this user's durable
    /// read position. Activity authored by the requester does not make their
    /// own channel unread.
    async fn list_unread_incident_ids(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, DomainError> {
        let _ = (team_id, user_id);
        Ok(Vec::new())
    }
    /// Advance one user's read position monotonically to content the client
    /// has actually loaded.
    async fn mark_incident_read(
        &self,
        incident_id: Uuid,
        user_id: Uuid,
        read_through: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError> {
        let _ = (incident_id, user_id, read_through);
        Ok(())
    }
    async fn delete_incident(&self, incident_id: Uuid) -> Result<(), DomainError>;
    /// Clear the assignee on every incident of `team_id` currently assigned to
    /// `user_id`. Called when a member is kicked/banned so no incident stays
    /// assigned to a non-member (upholds the assignee-must-be-a-member rule).
    async fn clear_assignee_for_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait TimelineRepo: Send + Sync {
    async fn append_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError>;
    /// Newest first, sharing the incident activity cursor with the event log so
    /// the two streams can be merged into one stable page.
    async fn list_entries_for_incident(
        &self,
        incident_id: Uuid,
        before: Option<ActivityCursor>,
        limit: u32,
    ) -> Result<Vec<TimelineEntry>, DomainError>;
    /// Load a single entry (to authorize and apply an edit).
    async fn find_entry_by_id(&self, entry_id: Uuid) -> Result<Option<TimelineEntry>, DomainError>;
    /// Persist an edited entry: updates `content` and `edited_at`.
    async fn update_entry(&self, entry: &TimelineEntry) -> Result<(), DomainError>;
    /// Add a reaction; returns `true` when newly inserted, `false` when the user
    /// already had that emoji on the entry (idempotent — no duplicate).
    async fn add_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, DomainError>;
    /// Remove a reaction (idempotent: removing a missing one is not an error).
    async fn remove_reaction(
        &self,
        entry_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<(), DomainError>;
    /// How many distinct users currently react to `entry_id` with `emoji`.
    async fn count_reaction(&self, entry_id: Uuid, emoji: &str) -> Result<u64, DomainError>;
    /// Every reaction on every entry of an incident, for roster aggregation.
    async fn list_reactions_for_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<ReactionRecord>, DomainError>;
    /// Load one attachment only when the requester still belongs to its Team.
    async fn find_attachment_for_member(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MessageAttachment>, DomainError>;
}

#[async_trait]
pub trait PrivateMessageRepo: Send + Sync {
    /// Persist a sent private message.
    async fn save(&self, message: &PrivateMessage) -> Result<(), DomainError>;
    /// The conversation between two users — both directions of the pair — newest
    /// first, capped at `limit`. The pair is symmetric, so the argument order
    /// does not matter.
    async fn list_conversation(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        before: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        limit: u32,
    ) -> Result<Vec<PrivateMessage>, DomainError>;
    async fn find_participants(
        &self,
        message_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, DomainError>;
    async fn update_content(
        &self,
        message_id: Uuid,
        content: &str,
        edited_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError>;
    async fn find_attachment_for_participant(
        &self,
        attachment_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PrivateMessageAttachment>, DomainError>;
    /// Persist (UPSERT) a viewer's read position for conversation with a peer.
    async fn mark_read(
        &self,
        viewer_id: Uuid,
        peer_id: Uuid,
        read_through: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError>;
    /// List all peer_ids with unread messages for viewer_id.
    async fn list_unread_peer_ids(&self, viewer_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
}

#[async_trait]
pub trait ReleaseRepo: Send + Sync {
    /// Persist a new release and all its (unvalidated) steps.
    async fn save_release(&self, release: &Release) -> Result<(), DomainError>;
    /// Persist a release and its normalized internal event atomically.
    async fn create_release(
        &self,
        release: &Release,
        delivery_id: &str,
        event: &crate::domain::automation::ExternalEvent,
    ) -> Result<(), DomainError>;
    /// Create an incident, its audit event and its Release link in one
    /// transaction, provided the Release snapshot is still current.
    async fn create_blocking_incident(
        &self,
        release_id: Uuid,
        expected_updated_at: chrono::DateTime<chrono::Utc>,
        incident: &Incident,
        event: &IncidentEvent,
    ) -> Result<(), DomainError>;
    /// Load a release with its ordered steps, or `None`.
    async fn find_release_by_id(&self, release_id: Uuid) -> Result<Option<Release>, DomainError>;
    /// Every release of a team (with steps), newest first.
    async fn list_releases_for_team(&self, team_id: Uuid) -> Result<Vec<Release>, DomainError>;
    /// Persist a mutated release and its steps only if the loaded snapshot is
    /// still current.
    async fn update_release(
        &self,
        release: &Release,
        expected_updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError>;
    /// Persist a validated Release step and every linked Incident audit event
    /// in the same transaction.
    async fn update_release_with_incident_events(
        &self,
        release: &Release,
        expected_updated_at: chrono::DateTime<chrono::Utc>,
        events: &[IncidentEvent],
    ) -> Result<(), DomainError>;
    /// Link an incident to a release (idempotent on the pair).
    async fn link_incident(&self, release_id: Uuid, incident_id: Uuid) -> Result<(), DomainError>;
    /// Unlink an incident from a release (idempotent: unlinking a missing pair is
    /// not an error).
    async fn unlink_incident(&self, release_id: Uuid, incident_id: Uuid)
        -> Result<(), DomainError>;
    /// The incidents currently linked to a release, for the read view.
    async fn list_linked_incident_ids(&self, release_id: Uuid) -> Result<Vec<Uuid>, DomainError>;
    /// How many of a release's linked incidents are still active (not resolved).
    /// `> 0` is exactly the "is it blocked?" input for `effective_release_state`.
    async fn count_active_linked_incidents(&self, release_id: Uuid) -> Result<u64, DomainError>;
    /// `(release_id, team_id, base_state)` of every release linked to an incident.
    /// Lets an incident status change recompute the blocking of affected releases
    /// without loading their full aggregates.
    async fn list_release_states_linked_to_incident(
        &self,
        incident_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, ReleaseBaseState)>, DomainError>;
}
