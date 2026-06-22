// --- server/src/adapters/ws/hub.rs ---

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::protocol::{presence_wire, team_presence_wire, to_wire};
use crate::domain::event::DomainEvent;
use crate::ports::EventPublisher;

pub type ConnectionId = Uuid;

struct Connection {
    user_id: Uuid,
    teams: HashSet<Uuid>,
    /// Incidents this connection is currently watching (presence). Ephemeral.
    watching: HashSet<Uuid>,
    tx: UnboundedSender<String>,
}

/// In-memory registry of live WebSocket connections. Implements `EventPublisher`
/// by fanning a domain event out to every connection whose user belongs to the
/// event's team. Ephemeral by design: presence and routing state live here, not
/// in the database.
#[derive(Default)]
pub struct WsHub {
    connections: Mutex<HashMap<ConnectionId, Connection>>,
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
        tx: UnboundedSender<String>,
    ) -> ConnectionId {
        let id = Uuid::new_v4();
        let team_ids: Vec<Uuid> = teams.iter().copied().collect();
        let mut conns = self.connections.lock().unwrap();
        conns.insert(
            id,
            Connection {
                user_id,
                teams,
                watching: HashSet::new(),
                tx,
            },
        );
        // The new connection is already in the map, so broadcasting now both
        // delivers its teams' current online rosters to it (no command needed)
        // and tells existing members it just came online.
        for team_id in team_ids {
            broadcast_team_presence(&conns, team_id);
        }
        id
    }

    pub fn unregister(&self, id: ConnectionId) {
        let mut conns = self.connections.lock().unwrap();
        if let Some(conn) = conns.remove(&id) {
            // The connection is already gone from the map, so rebroadcasting now
            // naturally drops it from every incident's watcher list and from each
            // of its teams' online rosters.
            for incident_id in conn.watching {
                broadcast_presence(&conns, incident_id);
            }
            for team_id in conn.teams {
                broadcast_team_presence(&conns, team_id);
            }
        }
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
            broadcast_team_presence(&conns, *team_id);
        }
    }

    /// Mark a connection as watching an incident and notify the co-watchers.
    pub fn watch(&self, conn_id: ConnectionId, incident_id: Uuid) {
        let mut conns = self.connections.lock().unwrap();
        let changed = conns
            .get_mut(&conn_id)
            .is_some_and(|c| c.watching.insert(incident_id));
        if changed {
            broadcast_presence(&conns, incident_id);
        }
    }

    /// Stop watching an incident and notify the remaining co-watchers.
    pub fn unwatch(&self, conn_id: ConnectionId, incident_id: Uuid) {
        let mut conns = self.connections.lock().unwrap();
        let changed = conns
            .get_mut(&conn_id)
            .is_some_and(|c| c.watching.remove(&incident_id));
        if changed {
            broadcast_presence(&conns, incident_id);
        }
    }

    #[cfg(test)]
    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }
}

/// Send a `presence_update` for `incident_id` to every connection watching it.
/// The watcher list is the *distinct* users currently watching (a user with two
/// tabs counts once). Called while holding the connections lock.
fn broadcast_presence(conns: &HashMap<ConnectionId, Connection>, incident_id: Uuid) {
    let mut watchers: Vec<Uuid> = conns
        .values()
        .filter(|c| c.watching.contains(&incident_id))
        .map(|c| c.user_id)
        .collect();
    watchers.sort();
    watchers.dedup();

    let payload = presence_wire(incident_id, &watchers);
    for conn in conns.values() {
        if conn.watching.contains(&incident_id) {
            let _ = conn.tx.send(payload.clone());
        }
    }
}

/// Send a `team_presence_update` for `team_id` to every connection in that team.
/// The online list is the *distinct* connected users who belong to the team (a
/// user with several tabs counts once). Strictly scoped: only members of the
/// team receive it, so a team's roster never leaks to outsiders. Called while
/// holding the connections lock.
fn broadcast_team_presence(conns: &HashMap<ConnectionId, Connection>, team_id: Uuid) {
    let mut online: Vec<Uuid> = conns
        .values()
        .filter(|c| c.teams.contains(&team_id))
        .map(|c| c.user_id)
        .collect();
    online.sort();
    online.dedup();

    let payload = team_presence_wire(team_id, &online);
    for conn in conns.values() {
        if conn.teams.contains(&team_id) {
            let _ = conn.tx.send(payload.clone());
        }
    }
}

#[async_trait]
impl EventPublisher for WsHub {
    async fn publish(&self, event: DomainEvent) {
        let team_id = event.team_id();
        let payload = to_wire(&event);
        // Collect-and-send under the lock is fine: UnboundedSender::send is
        // synchronous and non-blocking, so no await is held across the lock.
        let conns = self.connections.lock().unwrap();
        for conn in conns.values() {
            if conn.teams.contains(&team_id) {
                // Ignore send errors: a closed receiver means the connection is
                // gone and its task will unregister itself.
                let _ = conn.tx.send(payload.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::incident::IncidentStatus;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn publishes_only_to_connections_in_the_event_team() {
        let hub = WsHub::new();
        let team_a = Uuid::new_v4();
        let team_b = Uuid::new_v4();

        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        hub.register(Uuid::new_v4(), HashSet::from([team_a]), tx_a);
        hub.register(Uuid::new_v4(), HashSet::from([team_b]), tx_b);
        // Each registration broadcasts the team's online roster; drain those so
        // the assertion below sees only the incident event.
        while rx_a.try_recv().is_ok() {}
        while rx_b.try_recv().is_ok() {}

        hub.publish(DomainEvent::IncidentStateChanged {
            team_id: team_a,
            incident_id: Uuid::new_v4(),
            new_status: IncidentStatus::Acknowledged,
            by: Uuid::new_v4(),
        })
        .await;

        let msg = rx_a.try_recv().unwrap();
        assert!(msg.contains("incident_state_changed"));
        assert!(rx_b.try_recv().is_err());
    }

    #[tokio::test]
    async fn register_broadcasts_team_presence_to_the_new_connection() {
        let hub = WsHub::new();
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();
        hub.register(user, HashSet::from([team]), tx);

        let m = rx.try_recv().unwrap();
        assert!(m.contains("team_presence_update"));
        assert!(m.contains(&team.to_string()));
        assert!(m.contains(&user.to_string()));
    }

    #[tokio::test]
    async fn unregister_removes_user_from_team_presence() {
        let hub = WsHub::new();
        let team = Uuid::new_v4();
        let (user_a, user_b) = (Uuid::new_v4(), Uuid::new_v4());
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        let a = hub.register(user_a, HashSet::from([team]), tx_a);
        hub.register(user_b, HashSet::from([team]), tx_b);
        while rx_b.try_recv().is_ok() {} // drain register frames

        hub.unregister(a);
        let m = rx_b.try_recv().unwrap();
        assert!(m.contains("team_presence_update"));
        assert!(!m.contains(&user_a.to_string()));
        assert!(m.contains(&user_b.to_string()));
    }

    #[tokio::test]
    async fn team_presence_dedupes_a_user_with_multiple_tabs() {
        let hub = WsHub::new();
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        hub.register(user, HashSet::from([team]), tx1);
        hub.register(user, HashSet::from([team]), tx2);

        // The latest frame the first tab received (after the second tab joined)
        // lists the user exactly once.
        let mut last = None;
        while let Ok(m) = rx1.try_recv() {
            last = Some(m);
        }
        let v: serde_json::Value = serde_json::from_str(&last.unwrap()).unwrap();
        assert_eq!(v["online_user_ids"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn team_presence_does_not_leak_across_teams() {
        let hub = WsHub::new();
        let (team_a, team_b) = (Uuid::new_v4(), Uuid::new_v4());
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        hub.register(Uuid::new_v4(), HashSet::from([team_a]), tx_a);
        while rx_a.try_recv().is_ok() {} // drain A's own register frame

        // A user joins team_b: the team_a-only connection must hear nothing.
        let (tx_b, _rx_b) = mpsc::unbounded_channel();
        hub.register(Uuid::new_v4(), HashSet::from([team_b]), tx_b);
        assert!(rx_a.try_recv().is_err());
    }

    #[tokio::test]
    async fn refresh_teams_adds_the_user_to_a_newly_joined_team() {
        let hub = WsHub::new();
        let team = Uuid::new_v4();
        let user = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let conn = hub.register(user, HashSet::new(), tx); // starts with no teams
        while rx.try_recv().is_ok() {} // (nothing: no teams at register)

        hub.refresh_teams(conn, HashSet::from([team]));

        let m = rx.try_recv().unwrap();
        assert!(m.contains("team_presence_update"));
        assert!(m.contains(&team.to_string()));
        assert!(m.contains(&user.to_string()));
    }

    #[tokio::test]
    async fn refresh_teams_removes_the_user_from_a_left_team() {
        let hub = WsHub::new();
        let team = Uuid::new_v4();
        let (leaver, stayer) = (Uuid::new_v4(), Uuid::new_v4());
        let (tx_l, _rx_l) = mpsc::unbounded_channel();
        let (tx_s, mut rx_s) = mpsc::unbounded_channel();
        let leaver_conn = hub.register(leaver, HashSet::from([team]), tx_l);
        hub.register(stayer, HashSet::from([team]), tx_s);
        while rx_s.try_recv().is_ok() {} // drain register frames

        // The leaver refreshes to an empty team set (they left the team).
        hub.refresh_teams(leaver_conn, HashSet::new());

        let m = rx_s.try_recv().unwrap();
        assert!(m.contains("team_presence_update"));
        assert!(!m.contains(&leaver.to_string()));
        assert!(m.contains(&stayer.to_string()));
    }

    #[tokio::test]
    async fn a_user_in_two_teams_gets_presence_for_each_team() {
        let hub = WsHub::new();
        let (team_a, team_b) = (Uuid::new_v4(), Uuid::new_v4());
        let user = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel();
        hub.register(user, HashSet::from([team_a, team_b]), tx);

        let mut teams_seen = HashSet::new();
        while let Ok(m) = rx.try_recv() {
            let v: serde_json::Value = serde_json::from_str(&m).unwrap();
            if v["type"] == "team_presence_update" {
                teams_seen.insert(v["team_id"].as_str().unwrap().to_string());
            }
        }
        assert!(teams_seen.contains(&team_a.to_string()));
        assert!(teams_seen.contains(&team_b.to_string()));
    }

    #[tokio::test]
    async fn unregister_removes_the_connection() {
        let hub = WsHub::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let id = hub.register(Uuid::new_v4(), HashSet::from([Uuid::new_v4()]), tx);
        assert_eq!(hub.connection_count(), 1);

        hub.unregister(id);
        assert_eq!(hub.connection_count(), 0);
    }

    #[tokio::test]
    async fn watch_broadcasts_presence_to_co_watchers_only() {
        let hub = WsHub::new();
        let incident = Uuid::new_v4();
        let (user_a, user_b) = (Uuid::new_v4(), Uuid::new_v4());

        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        let a = hub.register(user_a, HashSet::new(), tx_a);
        let b = hub.register(user_b, HashSet::new(), tx_b);

        // A watches alone: A is notified, B (not watching) is not.
        hub.watch(a, incident);
        let m = rx_a.try_recv().unwrap();
        assert!(m.contains("presence_update"));
        assert!(m.contains(&user_a.to_string()));
        assert!(rx_b.try_recv().is_err());

        // B watches too: both now receive a presence update listing both users.
        hub.watch(b, incident);
        let m_a = rx_a.try_recv().unwrap();
        let m_b = rx_b.try_recv().unwrap();
        assert!(m_a.contains(&user_b.to_string()));
        assert!(m_b.contains(&user_a.to_string()));
    }

    #[tokio::test]
    async fn presence_is_scoped_to_the_watched_incident() {
        let hub = WsHub::new();
        let (incident_1, incident_2) = (Uuid::new_v4(), Uuid::new_v4());

        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();
        let a = hub.register(Uuid::new_v4(), HashSet::new(), tx_a);
        let b = hub.register(Uuid::new_v4(), HashSet::new(), tx_b);

        hub.watch(a, incident_1);
        let m = rx_a.try_recv().unwrap();
        assert!(m.contains(&incident_1.to_string()));

        // B watching a different incident must not reach A.
        hub.watch(b, incident_2);
        assert!(rx_a.try_recv().is_err());
    }

    #[tokio::test]
    async fn presence_dedupes_a_user_with_multiple_connections() {
        let hub = WsHub::new();
        let incident = Uuid::new_v4();
        let user = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let c1 = hub.register(user, HashSet::new(), tx1);
        let c2 = hub.register(user, HashSet::new(), tx2);

        hub.watch(c1, incident);
        hub.watch(c2, incident);

        // Take the latest presence frame c1 received: the user appears once.
        let mut last = None;
        while let Ok(m) = rx1.try_recv() {
            last = Some(m);
        }
        let v: serde_json::Value = serde_json::from_str(&last.unwrap()).unwrap();
        assert_eq!(v["watchers"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn disconnect_drops_the_user_from_presence() {
        let hub = WsHub::new();
        let incident = Uuid::new_v4();
        let user_a = Uuid::new_v4();

        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        let a = hub.register(user_a, HashSet::new(), tx_a);
        let b = hub.register(Uuid::new_v4(), HashSet::new(), tx_b);

        hub.watch(a, incident);
        hub.watch(b, incident);
        while rx_b.try_recv().is_ok() {} // drain join notifications

        // A disconnects: B is told A is gone.
        hub.unregister(a);
        let m = rx_b.try_recv().unwrap();
        assert!(m.contains("presence_update"));
        assert!(!m.contains(&user_a.to_string()));
    }
}
