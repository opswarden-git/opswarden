use super::*;
use crate::domain::incident::IncidentStatus;
use tokio::sync::mpsc;

#[tokio::test]
async fn publishes_only_to_connections_in_the_event_team() {
    let hub = WsHub::new();
    let team_a = Uuid::new_v4();
    let team_b = Uuid::new_v4();

    let (tx_a, mut rx_a) = mpsc::channel(256);
    let (tx_b, mut rx_b) = mpsc::channel(256);
    hub.register(Uuid::new_v4(), HashSet::from([team_a]), tx_a);
    hub.register(Uuid::new_v4(), HashSet::from([team_b]), tx_b);
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
async fn private_message_reaches_only_sender_and_recipient() {
    let hub = WsHub::new();
    let (sender, recipient, bystander) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let team = Uuid::new_v4();
    let (tx_s, mut rx_s) = mpsc::channel(256);
    let (tx_r, mut rx_r) = mpsc::channel(256);
    let (tx_b, mut rx_b) = mpsc::channel(256);
    hub.register(sender, HashSet::from([team]), tx_s);
    hub.register(recipient, HashSet::from([team]), tx_r);
    hub.register(bystander, HashSet::from([team]), tx_b);
    while rx_s.try_recv().is_ok() {}
    while rx_r.try_recv().is_ok() {}
    while rx_b.try_recv().is_ok() {}

    hub.publish(DomainEvent::PrivateMessageReceived {
        message_id: Uuid::new_v4(),
        sender_id: sender,
        recipient_id: recipient,
        content: "psst".to_string(),
        at: chrono::Utc::now(),
    })
    .await;

    assert!(rx_s
        .try_recv()
        .unwrap()
        .contains("private_message_received"));
    assert!(rx_r
        .try_recv()
        .unwrap()
        .contains("private_message_received"));
    assert!(rx_b.try_recv().is_err());
}

#[tokio::test]
async fn disconnects_a_connection_when_its_outbound_queue_is_full() {
    let hub = WsHub::new();
    let user = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel(1);
    hub.register(user, HashSet::new(), tx);

    let event = || DomainEvent::PrivateMessageReceived {
        message_id: Uuid::new_v4(),
        sender_id: user,
        recipient_id: Uuid::new_v4(),
        content: "bounded".to_string(),
        at: chrono::Utc::now(),
    };
    hub.publish(event()).await;
    assert_eq!(hub.connection_count(), 1);

    hub.publish(event()).await;
    assert_eq!(hub.connection_count(), 0);
}

#[tokio::test]
async fn register_broadcasts_team_presence_to_the_new_connection() {
    let hub = WsHub::new();
    let team = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (tx, mut rx) = mpsc::channel(256);
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
    let (tx_a, _rx_a) = mpsc::channel(256);
    let (tx_b, mut rx_b) = mpsc::channel(256);
    let a = hub.register(user_a, HashSet::from([team]), tx_a);
    hub.register(user_b, HashSet::from([team]), tx_b);
    while rx_b.try_recv().is_ok() {}

    hub.unregister(a);
    let m = rx_b.try_recv().unwrap();
    assert!(m.contains("team_presence_update"));
    assert!(!m.contains(&user_a.to_string()));
    assert!(m.contains(&user_b.to_string()));
}

#[tokio::test]
async fn disconnect_user_closes_every_tab_but_not_other_users() {
    let hub = WsHub::new();
    let team = Uuid::new_v4();
    let (user, other) = (Uuid::new_v4(), Uuid::new_v4());
    let (tx_a, mut rx_a) = mpsc::channel(256);
    let (tx_b, mut rx_b) = mpsc::channel(256);
    let (tx_other, mut rx_other) = mpsc::channel(256);
    hub.register(user, HashSet::from([team]), tx_a);
    hub.register(user, HashSet::from([team]), tx_b);
    hub.register(other, HashSet::from([team]), tx_other);
    while rx_a.try_recv().is_ok() {}
    while rx_b.try_recv().is_ok() {}
    while rx_other.try_recv().is_ok() {}

    hub.disconnect_user(user);

    assert_eq!(hub.connection_count(), 1);
    assert!(rx_a.recv().await.is_none());
    assert!(rx_b.recv().await.is_none());
    let presence = rx_other.try_recv().unwrap();
    assert!(!presence.contains(&user.to_string()));
    assert!(presence.contains(&other.to_string()));
}

#[tokio::test]
async fn disconnect_team_member_is_scoped_to_the_revoked_membership() {
    let hub = WsHub::new();
    let (revoked_team, other_team) = (Uuid::new_v4(), Uuid::new_v4());
    let user = Uuid::new_v4();
    let (revoked_tx, mut revoked_rx) = mpsc::channel(256);
    let (other_tx, mut other_rx) = mpsc::channel(256);
    hub.register(user, HashSet::from([revoked_team]), revoked_tx);
    hub.register(user, HashSet::from([other_team]), other_tx);
    while revoked_rx.try_recv().is_ok() {}
    while other_rx.try_recv().is_ok() {}

    hub.disconnect_team_member(revoked_team, user);

    assert_eq!(hub.connection_count(), 1);
    assert!(revoked_rx.recv().await.is_none());
    assert!(matches!(
        other_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn disconnect_team_closes_members_but_not_outsiders() {
    let hub = WsHub::new();
    let deleted_team = Uuid::new_v4();
    let (member_tx, mut member_rx) = mpsc::channel(256);
    let (outsider_tx, mut outsider_rx) = mpsc::channel(256);
    hub.register(Uuid::new_v4(), HashSet::from([deleted_team]), member_tx);
    hub.register(Uuid::new_v4(), HashSet::new(), outsider_tx);
    while member_rx.try_recv().is_ok() {}

    hub.disconnect_team(deleted_team);

    assert_eq!(hub.connection_count(), 1);
    assert!(member_rx.recv().await.is_none());
    assert!(matches!(
        outsider_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn team_presence_dedupes_a_user_with_multiple_tabs() {
    let hub = WsHub::new();
    let team = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (tx1, mut rx1) = mpsc::channel(256);
    let (tx2, _rx2) = mpsc::channel(256);
    hub.register(user, HashSet::from([team]), tx1);
    hub.register(user, HashSet::from([team]), tx2);

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
    let (tx_a, mut rx_a) = mpsc::channel(256);
    hub.register(Uuid::new_v4(), HashSet::from([team_a]), tx_a);
    while rx_a.try_recv().is_ok() {}

    let (tx_b, _rx_b) = mpsc::channel(256);
    hub.register(Uuid::new_v4(), HashSet::from([team_b]), tx_b);
    assert!(rx_a.try_recv().is_err());
}

#[tokio::test]
async fn refresh_teams_adds_the_user_to_a_newly_joined_team() {
    let hub = WsHub::new();
    let team = Uuid::new_v4();
    let user = Uuid::new_v4();
    let (tx, mut rx) = mpsc::channel(256);
    let conn = hub.register(user, HashSet::new(), tx);
    while rx.try_recv().is_ok() {}

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
    let (tx_l, _rx_l) = mpsc::channel(256);
    let (tx_s, mut rx_s) = mpsc::channel(256);
    let leaver_conn = hub.register(leaver, HashSet::from([team]), tx_l);
    hub.register(stayer, HashSet::from([team]), tx_s);
    while rx_s.try_recv().is_ok() {}

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
    let (tx, mut rx) = mpsc::channel(256);
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
    let (tx, _rx) = mpsc::channel(256);
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

    let (tx_a, mut rx_a) = mpsc::channel(256);
    let (tx_b, mut rx_b) = mpsc::channel(256);
    let a = hub.register(user_a, HashSet::new(), tx_a);
    let b = hub.register(user_b, HashSet::new(), tx_b);

    hub.watch(a, incident);
    let m = rx_a.try_recv().unwrap();
    assert!(m.contains("presence_update"));
    assert!(m.contains(&user_a.to_string()));
    assert!(rx_b.try_recv().is_err());

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

    let (tx_a, mut rx_a) = mpsc::channel(256);
    let (tx_b, _rx_b) = mpsc::channel(256);
    let a = hub.register(Uuid::new_v4(), HashSet::new(), tx_a);
    let b = hub.register(Uuid::new_v4(), HashSet::new(), tx_b);

    hub.watch(a, incident_1);
    let m = rx_a.try_recv().unwrap();
    assert!(m.contains(&incident_1.to_string()));

    hub.watch(b, incident_2);
    assert!(rx_a.try_recv().is_err());
}

#[tokio::test]
async fn presence_dedupes_a_user_with_multiple_connections() {
    let hub = WsHub::new();
    let incident = Uuid::new_v4();
    let user = Uuid::new_v4();

    let (tx1, mut rx1) = mpsc::channel(256);
    let (tx2, _rx2) = mpsc::channel(256);
    let c1 = hub.register(user, HashSet::new(), tx1);
    let c2 = hub.register(user, HashSet::new(), tx2);

    hub.watch(c1, incident);
    hub.watch(c2, incident);

    let mut last = None;
    while let Ok(m) = rx1.try_recv() {
        last = Some(m);
    }
    let v: serde_json::Value = serde_json::from_str(&last.unwrap()).unwrap();
    assert_eq!(v["watchers"].as_array().unwrap().len(), 1);
}
