// --- server/src/adapters/ws/hub.rs ---

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::protocol::to_wire;
use crate::domain::event::DomainEvent;
use crate::ports::EventPublisher;

pub type ConnectionId = Uuid;

struct Connection {
    // Read by the presence / private-message fan-out in PR2.
    #[allow(dead_code)]
    user_id: Uuid,
    teams: HashSet<Uuid>,
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
        self.connections
            .lock()
            .unwrap()
            .insert(id, Connection { user_id, teams, tx });
        id
    }

    pub fn unregister(&self, id: ConnectionId) {
        self.connections.lock().unwrap().remove(&id);
    }

    #[cfg(test)]
    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
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
}
