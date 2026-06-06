// --- server/src/adapters/ws/hub.rs ---

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::protocol::{presence_wire, to_wire};
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
        self.connections.lock().unwrap().insert(
            id,
            Connection {
                user_id,
                teams,
                watching: HashSet::new(),
                tx,
            },
        );
        id
    }

    pub fn unregister(&self, id: ConnectionId) {
        let mut conns = self.connections.lock().unwrap();
        if let Some(conn) = conns.remove(&id) {
            // The connection is already gone from the map, so rebroadcasting now
            // naturally drops it from every incident's watcher list.
            for incident_id in conn.watching {
                broadcast_presence(&conns, incident_id);
            }
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
