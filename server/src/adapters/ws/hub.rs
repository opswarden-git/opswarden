// --- server/src/adapters/ws/hub.rs ---

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc::{error::TrySendError, Sender};
use uuid::Uuid;

use super::hub_rooms::{broadcast_room_presence, RoomKey};
use super::protocol::{cursor_wire, team_presence_wire, to_wire};
use crate::domain::event::{DomainEvent, EventDelivery};
use crate::ports::EventPublisher;

pub type ConnectionId = Uuid;

/// Maximum number of serialized frames buffered for one WebSocket connection.
/// A connection that cannot keep up is disconnected instead of being allowed
/// to grow the server's memory without bound.
pub const OUTBOUND_QUEUE_CAPACITY: usize = 256;

pub(super) struct Connection {
    pub(super) user_id: Uuid,
    teams: HashSet<Uuid>,
    /// Incidents this connection is currently watching (presence). Ephemeral.
    /// Incident and bilateral rooms actively open on this connection.
    pub(super) rooms: HashSet<RoomKey>,
    pub(super) tx: Sender<String>,
}

/// In-memory registry of live WebSocket connections and presence.
#[derive(Default)]
pub struct WsHub {
    pub(super) connections: Mutex<HashMap<ConnectionId, Connection>>,
}

impl WsHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a connected client; returns the id used to `unregister` on close.
    pub fn register(
        &self,
        user_id: Uuid,
        teams: HashSet<Uuid>,
        tx: Sender<String>,
    ) -> ConnectionId {
        let id = Uuid::new_v4();
        let team_ids: Vec<Uuid> = teams.iter().copied().collect();
        let mut conns = self.connections.lock().unwrap();
        conns.insert(
            id,
            Connection {
                user_id,
                teams,
                rooms: HashSet::new(),
                tx,
            },
        );
        // The new connection is already in the map, so broadcasting now both
        // delivers its teams' current online rosters to it (no command needed)
        // and tells existing members it just came online.
        for team_id in team_ids {
            broadcast_team_presence(&mut conns, team_id);
        }
        id
    }

    pub fn unregister(&self, id: ConnectionId) {
        let mut conns = self.connections.lock().unwrap();
        remove_connections(&mut conns, [id]);
    }

    /// Close every live socket for an account. Used after logout and account
    /// deletion so an already-open WebSocket cannot outlive the HTTP session
    /// revocation that triggered the operation.
    pub fn disconnect_user(&self, user_id: Uuid) {
        let mut conns = self.connections.lock().unwrap();
        let ids = conns
            .iter()
            .filter_map(|(id, conn)| (conn.user_id == user_id).then_some(*id))
            .collect::<Vec<_>>();
        remove_connections(&mut conns, ids);
    }

    /// Close this member's sockets that still carry the revoked team scope.
    /// Other users and any connection that never had this team remain intact.
    pub fn disconnect_team_member(&self, team_id: Uuid, user_id: Uuid) {
        let mut conns = self.connections.lock().unwrap();
        let ids = conns
            .iter()
            .filter_map(|(id, conn)| {
                (conn.user_id == user_id && conn.teams.contains(&team_id)).then_some(*id)
            })
            .collect::<Vec<_>>();
        remove_connections(&mut conns, ids);
    }

    /// Close every socket scoped to a deleted team. Affected users may retain
    /// valid HTTP sessions and reconnect with their remaining memberships.
    pub fn disconnect_team(&self, team_id: Uuid) {
        let mut conns = self.connections.lock().unwrap();
        let ids = conns
            .iter()
            .filter_map(|(id, conn)| conn.teams.contains(&team_id).then_some(*id))
            .collect::<Vec<_>>();
        remove_connections(&mut conns, ids);
    }

    /// Replace a connection's team scope (after the user created/joined/left/
    /// deleted a team) and re-broadcast presence for every team the change
    /// touched. The new set is resolved from the database by the caller, so this
    /// stays a pure in-memory swap. Only the *changed* teams are re-broadcast: a
    /// left team drops the user (its remaining members are notified, this
    /// connection no longer is), a joined team gains the user (and this
    /// connection now receives that team's roster).
    pub fn refresh_teams(&self, conn_id: ConnectionId, new_teams: HashSet<Uuid>) {
        let mut conns = self.connections.lock().unwrap();
        let old_teams = match conns.get_mut(&conn_id) {
            Some(conn) => std::mem::replace(&mut conn.teams, new_teams.clone()),
            None => return,
        };
        for team_id in old_teams.symmetric_difference(&new_teams) {
            broadcast_team_presence(&mut conns, *team_id);
        }
    }

    /// Relay a pointer only when its connection already watches the incident.
    /// The authorized `watch` command therefore remains the single admission
    /// gate, while high-frequency pointer frames never hit the database.
    pub fn cursor(&self, conn_id: ConnectionId, incident_id: Uuid, x: f64, y: f64) {
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
        {
            return;
        }
        let mut conns = self.connections.lock().unwrap();
        let Some(source) = conns.get(&conn_id) else {
            return;
        };
        let room = RoomKey::Incident(incident_id);
        if !source.rooms.contains(&room) {
            return;
        }
        let payload = cursor_wire(incident_id, source.user_id, x, y);
        let recipients = conns
            .iter()
            .filter_map(|(recipient_id, conn)| {
                (*recipient_id != conn_id && conn.rooms.contains(&room)).then_some(*recipient_id)
            })
            .collect();
        deliver(&mut conns, recipients, &payload);
    }

    #[cfg(test)]
    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }
}

/// Send a `team_presence_update` for `team_id` to every connection in that team.
/// The online list is the *distinct* connected users who belong to the team (a
/// user with several tabs counts once). Strictly scoped: only members of the
/// team receive it, so a team's roster never leaks to outsiders. Called while
/// holding the connections lock.
fn broadcast_team_presence(conns: &mut HashMap<ConnectionId, Connection>, team_id: Uuid) {
    let mut online: Vec<Uuid> = conns
        .values()
        .filter(|c| c.teams.contains(&team_id))
        .map(|c| c.user_id)
        .collect();
    online.sort();
    online.dedup();

    let payload = team_presence_wire(team_id, &online);
    let recipients = conns
        .iter()
        .filter_map(|(id, conn)| conn.teams.contains(&team_id).then_some(*id))
        .collect();
    deliver(conns, recipients, &payload);
}

/// Queue one payload without blocking the publisher. Closed or saturated
/// recipients are removed immediately; dropping their last sender closes the
/// handler's receive loop and therefore the socket. Presence is then
/// re-broadcast for every room/team affected by the removal.
pub(super) fn deliver(
    conns: &mut HashMap<ConnectionId, Connection>,
    recipients: Vec<ConnectionId>,
    payload: &str,
) {
    let failed: Vec<ConnectionId> = recipients
        .into_iter()
        .filter(|id| {
            conns.get(id).is_some_and(|conn| {
                matches!(
                    conn.tx.try_send(payload.to_owned()),
                    Err(TrySendError::Full(_) | TrySendError::Closed(_))
                )
            })
        })
        .collect();
    if failed.is_empty() {
        return;
    }

    remove_connections(conns, failed);
}

fn remove_connections(
    conns: &mut HashMap<ConnectionId, Connection>,
    ids: impl IntoIterator<Item = ConnectionId>,
) {
    let mut rooms = HashSet::new();
    let mut teams = HashSet::new();
    for id in ids {
        if let Some(conn) = conns.remove(&id) {
            rooms.extend(conn.rooms);
            teams.extend(conn.teams);
        }
    }
    for room in rooms {
        broadcast_room_presence(conns, room);
    }
    for team_id in teams {
        broadcast_team_presence(conns, team_id);
    }
}

#[async_trait]
impl EventPublisher for WsHub {
    async fn publish(&self, event: DomainEvent) {
        let delivery = event.delivery();
        let payload = to_wire(&event);
        // Bounded `try_send` is synchronous, so no await is held across the
        // lock. Saturated clients are disconnected by `deliver`.
        let mut conns = self.connections.lock().unwrap();
        // A connection is a recipient when it belongs to the event's team
        // (team-scoped events) or when its user is one of the targeted users
        // (private messages). Closed and saturated receivers are removed.
        let recipients = conns
            .iter()
            .filter_map(|(id, conn)| {
                let is_recipient = match &delivery {
                    EventDelivery::Team(team_id) => conn.teams.contains(team_id),
                    EventDelivery::Users(user_ids) => user_ids.contains(&conn.user_id),
                };
                is_recipient.then_some(*id)
            })
            .collect();
        deliver(&mut conns, recipients, &payload);
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "hub_cursor_tests.rs"]
mod cursor_tests;
