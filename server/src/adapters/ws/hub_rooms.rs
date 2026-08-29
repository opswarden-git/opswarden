use std::collections::HashMap;

use uuid::Uuid;

use super::hub::{deliver, Connection, ConnectionId, WsHub};
use super::protocol::{
    presence_wire, private_message_presence_wire, private_message_typing_wire, user_typing_wire,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct PrivateRoom(Uuid, Uuid);

impl PrivateRoom {
    fn new(a: Uuid, b: Uuid) -> Self {
        if a < b {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }

    fn participants(self) -> [Uuid; 2] {
        [self.0, self.1]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RoomKey {
    Incident(Uuid),
    Direct(PrivateRoom),
}

impl WsHub {
    fn watch_room(&self, conn_id: ConnectionId, room: RoomKey) {
        let mut connections = self.connections.lock().unwrap();
        if connections
            .get_mut(&conn_id)
            .is_some_and(|connection| connection.rooms.insert(room))
        {
            broadcast_room_presence(&mut connections, room);
        }
    }

    fn unwatch_room(&self, conn_id: ConnectionId, room: RoomKey) {
        let mut connections = self.connections.lock().unwrap();
        if connections
            .get_mut(&conn_id)
            .is_some_and(|connection| connection.rooms.remove(&room))
        {
            broadcast_room_presence(&mut connections, room);
        }
    }

    pub fn watch(&self, conn_id: ConnectionId, incident_id: Uuid) {
        self.watch_room(conn_id, RoomKey::Incident(incident_id));
    }

    pub fn unwatch(&self, conn_id: ConnectionId, incident_id: Uuid) {
        self.unwatch_room(conn_id, RoomKey::Incident(incident_id));
    }

    pub fn watch_private_message(&self, conn_id: ConnectionId, peer_id: Uuid) {
        let Some(user_id) = self.connection_user(conn_id) else {
            return;
        };
        self.watch_room(conn_id, RoomKey::Direct(PrivateRoom::new(user_id, peer_id)));
    }

    pub fn unwatch_private_message(&self, conn_id: ConnectionId, peer_id: Uuid) {
        let Some(user_id) = self.connection_user(conn_id) else {
            return;
        };
        self.unwatch_room(conn_id, RoomKey::Direct(PrivateRoom::new(user_id, peer_id)));
    }

    pub fn typing(&self, conn_id: ConnectionId, incident_id: Uuid) {
        self.broadcast_typing(conn_id, RoomKey::Incident(incident_id));
    }

    pub fn private_message_typing(&self, conn_id: ConnectionId, peer_id: Uuid) {
        let Some(user_id) = self.connection_user(conn_id) else {
            return;
        };
        self.broadcast_typing(conn_id, RoomKey::Direct(PrivateRoom::new(user_id, peer_id)));
    }

    fn connection_user(&self, conn_id: ConnectionId) -> Option<Uuid> {
        self.connections
            .lock()
            .unwrap()
            .get(&conn_id)
            .map(|connection| connection.user_id)
    }

    fn broadcast_typing(&self, conn_id: ConnectionId, room: RoomKey) {
        let mut connections = self.connections.lock().unwrap();
        let Some(source) = connections.get(&conn_id) else {
            return;
        };
        if !source.rooms.contains(&room) {
            return;
        }
        let payload = match room {
            RoomKey::Incident(incident_id) => user_typing_wire(incident_id, source.user_id),
            RoomKey::Direct(private) => {
                let peer = private
                    .participants()
                    .into_iter()
                    .find(|participant| *participant != source.user_id)
                    .unwrap_or(source.user_id);
                private_message_typing_wire(source.user_id, peer)
            }
        };
        let recipients = connections
            .iter()
            .filter_map(|(recipient_id, connection)| {
                (*recipient_id != conn_id && connection.rooms.contains(&room))
                    .then_some(*recipient_id)
            })
            .collect();
        deliver(&mut connections, recipients, &payload);
    }
}

pub(super) fn broadcast_room_presence(
    connections: &mut HashMap<ConnectionId, Connection>,
    room: RoomKey,
) {
    let mut watchers: Vec<Uuid> = connections
        .values()
        .filter(|connection| connection.rooms.contains(&room))
        .map(|connection| connection.user_id)
        .collect();
    watchers.sort();
    watchers.dedup();
    let payload = match room {
        RoomKey::Incident(incident_id) => presence_wire(incident_id, "incident", &watchers),
        RoomKey::Direct(private) => {
            private_message_presence_wire(private.participants(), &watchers)
        }
    };
    let recipients = connections
        .iter()
        .filter_map(|(id, connection)| connection.rooms.contains(&room).then_some(*id))
        .collect();
    deliver(connections, recipients, &payload);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn direct_presence_and_typing_are_bilateral() {
        let hub = WsHub::new();
        let (alice, bob, outsider) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (alice_tx, mut alice_rx) = mpsc::channel(256);
        let (bob_tx, mut bob_rx) = mpsc::channel(256);
        let (outsider_tx, mut outsider_rx) = mpsc::channel(256);
        let alice_connection = hub.register(alice, HashSet::new(), alice_tx);
        let bob_connection = hub.register(bob, HashSet::new(), bob_tx);
        hub.register(outsider, HashSet::new(), outsider_tx);

        hub.watch_private_message(alice_connection, bob);
        alice_rx.try_recv().unwrap();
        hub.watch_private_message(bob_connection, alice);
        alice_rx.try_recv().unwrap();
        bob_rx.try_recv().unwrap();
        hub.private_message_typing(alice_connection, bob);

        let frame: serde_json::Value = serde_json::from_str(&bob_rx.try_recv().unwrap()).unwrap();
        assert_eq!(frame["type"], "private_message_typing");
        assert!(alice_rx.try_recv().is_err());
        assert!(outsider_rx.try_recv().is_err());
    }

    #[test]
    fn disconnect_updates_every_room_once() {
        let hub = WsHub::new();
        let (alice, bob) = (Uuid::new_v4(), Uuid::new_v4());
        let (alice_tx, mut alice_rx) = mpsc::channel(256);
        let (bob_tx, mut bob_rx) = mpsc::channel(256);
        let alice_connection = hub.register(alice, HashSet::new(), alice_tx);
        let bob_connection = hub.register(bob, HashSet::new(), bob_tx);
        hub.watch_private_message(alice_connection, bob);
        alice_rx.try_recv().unwrap();
        hub.watch_private_message(bob_connection, alice);
        alice_rx.try_recv().unwrap();
        bob_rx.try_recv().unwrap();

        hub.unregister(bob_connection);

        let frame: serde_json::Value = serde_json::from_str(&alice_rx.try_recv().unwrap()).unwrap();
        assert_eq!(frame["watchers"], serde_json::json!([alice]));
    }
}
