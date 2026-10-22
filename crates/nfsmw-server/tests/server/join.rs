//! One client through the whole flow: join, lobby, ready, countdown, driving, snapshots, acknowledgement, pings.

use nfsmw_net::input::InputCode;
use nfsmw_net::message::{Message, Role};
use nfsmw_net::schema::{NITROUS, RPM};
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config, throttle};

#[test]
fn a_client_joins_readies_drives_and_gets_acknowledged_deltas() {
    let mut h = Harness::new(test_config(), ToyConfig::default());
    let me = h.connect(10);
    h.join_all(&[me]);
    let accepted = h.clients[me].accepted.clone().expect("seated");
    assert_eq!(accepted.slot, 0);
    assert_eq!(accepted.snapshot_hz, 20);
    assert_eq!(accepted.server_caps.schema_id, h.clients[me].schema.schema_id(), "same schema on both sides");
    assert_eq!(accepted.resume_token.len(), 32);
    h.run(5, InputCode::NEUTRAL);
    let lobby = h.clients[me].messages.iter().find_map(|message| match message {
        Message::LobbyState(state) => Some(state.clone()),
        _ => None,
    });
    let lobby = lobby.expect("the lobby state on join");
    assert_eq!(lobby.players.len(), 1);
    assert_eq!(lobby.players[0].role, Role::Racer);

    h.start_race(&[me]);
    assert!(h.clients[me].got(|message| matches!(message, Message::StartCountdown(_))), "a start countdown");
    assert!(h.clients[me].got(|message| matches!(message, Message::LoadWorld(_))), "a load order");
    h.clients[me].messages.clear();
    let before = h.clients[me].snapshots.len();
    h.run(240, throttle());

    let client = &h.clients[me];
    assert_eq!(client.snapshot_errors, 0, "every snapshot decodes against its baseline");
    let fresh = &client.snapshots[before..];
    assert!(fresh.len() >= 60, "20 Hz for 4 s: {} snapshots", fresh.len());
    let deltas = fresh.iter().filter(|snapshot| snapshot.baseline_distance > 0).count();
    assert!(deltas * 10 >= fresh.len() * 8, "most snapshots are deltas: {deltas} of {}", fresh.len());
    let newest = fresh.last().expect("snapshots");
    let ack = client.acked.expect("inputs acknowledged");
    assert!(newest.tick.saturating_sub(ack) <= 6, "the server applied recent inputs: ack {ack}, tick {}", newest.tick);

    let mut compared = 0;
    for snapshot in fresh {
        let own = snapshot.own.expect("a driver gets its own block");
        let Some(world) = h.own.get(&(0, snapshot.tick)) else {
            continue;
        };
        assert!((own.own.rpm - world.rpm).abs() <= RPM.step(), "rpm {} against {}", own.own.rpm, world.rpm);
        assert_eq!(own.own.gear, world.gear.min(15));
        assert!((own.own.nitrous - world.nitrous).abs() <= NITROUS.step());
        assert_eq!(own.own.flags, world.flags);
        compared += 1;
    }
    assert!(compared >= 60, "own blocks compared: {compared}");
    let first = fresh.first().and_then(|snapshot| snapshot.own).expect("own block");
    let last = newest.own.expect("own block");
    let moved = (last.position[0] - first.position[0]).hypot(last.position[2] - first.position[2]);
    assert!(moved > 5.0, "full throttle moved the car {moved} m");
    assert!(last.report.on_time >= 8, "inputs arrive in time: {:?}", last.report);
    assert!(!client.pongs.is_empty(), "pings are answered");
    assert_eq!(h.server.stats().snapshot_errors, 0);
}

#[test]
fn a_second_join_from_the_same_connection_is_a_strike_not_a_seat() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let me = h.connect(11);
    h.join_all(&[me]);
    h.clients[me].join("again", 0, None, 0);
    h.run(20, InputCode::NEUTRAL);
    assert_eq!(h.server.lobby().members().count(), 1);
    assert!(h.server.stats().strikes >= 2, "Auth and Join out of order");
}
