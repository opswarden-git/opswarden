use std::collections::HashSet;

use tokio::sync::mpsc;
use uuid::Uuid;

use super::WsHub;

#[tokio::test]
async fn cursor_reaches_only_other_watchers_of_the_same_incident() {
    let hub = WsHub::new();
    let incident = Uuid::new_v4();
    let other_incident = Uuid::new_v4();
    let user_a = Uuid::new_v4();

    let (tx_a, mut rx_a) = mpsc::unbounded_channel();
    let (tx_b, mut rx_b) = mpsc::unbounded_channel();
    let (tx_c, mut rx_c) = mpsc::unbounded_channel();
    let a = hub.register(user_a, HashSet::new(), tx_a);
    let b = hub.register(Uuid::new_v4(), HashSet::new(), tx_b);
    let c = hub.register(Uuid::new_v4(), HashSet::new(), tx_c);
    hub.watch(a, incident);
    hub.watch(b, incident);
    hub.watch(c, other_incident);
    while rx_a.try_recv().is_ok() {}
    while rx_b.try_recv().is_ok() {}
    while rx_c.try_recv().is_ok() {}

    hub.cursor(a, incident, 0.25, 0.75);

    assert!(rx_a.try_recv().is_err());
    let value: serde_json::Value = serde_json::from_str(&rx_b.try_recv().unwrap()).unwrap();
    assert_eq!(value["type"], "cursor_update");
    assert_eq!(value["user_id"], user_a.to_string());
    assert!(rx_c.try_recv().is_err());
}

#[tokio::test]
async fn cursor_rejects_unwatched_or_invalid_positions() {
    let hub = WsHub::new();
    let incident = Uuid::new_v4();
    let (tx_a, _rx_a) = mpsc::unbounded_channel();
    let (tx_b, mut rx_b) = mpsc::unbounded_channel();
    let a = hub.register(Uuid::new_v4(), HashSet::new(), tx_a);
    let b = hub.register(Uuid::new_v4(), HashSet::new(), tx_b);
    hub.watch(b, incident);
    while rx_b.try_recv().is_ok() {}

    hub.cursor(a, incident, 0.5, 0.5);
    assert!(rx_b.try_recv().is_err());
    hub.watch(a, incident);
    while rx_b.try_recv().is_ok() {}
    hub.cursor(a, incident, 1.5, 0.5);
    assert!(rx_b.try_recv().is_err());
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
    while rx_b.try_recv().is_ok() {}

    hub.unregister(a);
    let message = rx_b.try_recv().unwrap();
    assert!(message.contains("presence_update"));
    assert!(!message.contains(&user_a.to_string()));
}
