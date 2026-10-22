//! A joiner of a running race watches it as a spectator (MP-23, MP-41, MP-53).

use nfsmw_net::drop::{BulkReceiver, SESSION_DATA_LIMIT, SessionData};
use nfsmw_net::message::{Message, Role};
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config, throttle};

#[test]
fn a_joiner_of_a_running_race_becomes_a_spectator_with_the_session_data() {
    let mut config = test_config();
    for (key, value) in [("mode", "circuit"), ("laps", "2"), ("join_in_progress", "between_races")] {
        nfsmw_net::settings::set(&mut config.settings, key, value).expect("a valid setting");
    }
    let mut h = Harness::new(config, ToyConfig::default());
    let racers: Vec<usize> = (0..2).map(|seed| h.connect(50 + seed)).collect();
    h.join_all(&racers);
    h.start_race(&racers);
    h.run(30, throttle());

    let late = h.connect(60);
    h.join_all(&[late]);
    h.run(120, throttle());
    let slot = h.clients[late].slot().expect("seated");
    let member = h.server.lobby().member(slot).expect("a member");
    assert_eq!(member.role, Role::Spectator, "a running race seats a joiner as a spectator");
    assert_eq!(member.wants, Role::Racer, "who races next time");
    assert_eq!(h.clients[late].accepted.as_ref().map(|accepted| accepted.snapshot_hz), Some(10));

    let mut bulk = BulkReceiver::new(SESSION_DATA_LIMIT);
    let mut data = None;
    for message in &h.clients[late].messages {
        if let Ok(Some(complete)) = bulk.accept(message) {
            data = Some(SessionData::decode(&complete.data).expect("valid session data"));
        }
    }
    let data = data.expect("the session data arrived on the bulk channel");
    assert_eq!(data.lobby.players.len(), 3);
    assert_eq!(data.loadouts.len(), 3);

    let client = &h.clients[late];
    assert_eq!(client.snapshot_errors, 0);
    assert!(client.snapshots.len() >= 15, "10 Hz for 2 s: {}", client.snapshots.len());
    assert!(client.snapshots.iter().all(|snapshot| snapshot.own.is_none()), "a spectator has no car");
    let view = client.views.latest().expect("a view");
    assert!(view.len() >= 2 + 10, "the racers, traffic and police: {}", view.len());
    assert!(!h.clients[late].got(|message| matches!(message, Message::LoadWorld(_))), "no load order for a watcher");
}
