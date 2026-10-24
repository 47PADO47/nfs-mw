//! A knockout round on the toy circuit: the slower of two drivers is last at the end of the first lap, the server
//! knocks them out and tells everyone with a `KnockedOut` naming their car, then the round ends with one racer left.

use nfsmw_net::input::InputCode;
use nfsmw_net::message::Message;
use nfsmw_sim::toy::{RaceConfig, ToyConfig};

use super::harness::{Harness, test_config};
use super::results::steer_paced;

#[test]
fn the_last_driver_of_a_lap_is_knocked_out_and_everyone_is_told() {
    let mut config = test_config();
    for (key, value) in [("mode", "knockout"), ("laps", "2")] {
        nfsmw_net::settings::set(&mut config.settings, key, value).expect("a valid setting");
    }
    let race = RaceConfig::circuit(2);
    let checkpoints = race.checkpoints.clone();
    let mut h = Harness::new(config, ToyConfig { race: Some(race), ..ToyConfig::bare() });
    let racers: Vec<usize> = (0..2).map(|seed| h.connect(110 + seed)).collect();
    h.join_all(&racers);
    h.start_race(&racers);
    let slow = racers[1];
    // A player's car is the entity with its slot's number.
    let slow_entity = u16::from(h.clients[slow].slot().expect("seated"));

    let mut next = vec![0usize; racers.len()];
    for _ in 0..12_000 {
        if h.clients[racers[0]].got(|message| matches!(message, Message::KnockedOut(_))) {
            break;
        }
        h.step_by(|index, client| {
            let Some(own) = client.snapshots.last().and_then(|snapshot| snapshot.own) else {
                return InputCode::NEUTRAL;
            };
            let target = checkpoints[next[index] % checkpoints.len()];
            if (target[0] - own.position[0]).hypot(target[1] - own.position[2]) < 15.0 {
                next[index] += 1;
            }
            let pace = if index == 1 { 0.5 } else { 1.0 };
            steer_paced(client, checkpoints[next[index] % checkpoints.len()], pace)
        });
    }
    for index in racers {
        let out = h.clients[index].messages.iter().find_map(|message| match message {
            Message::KnockedOut(out) => Some(out.clone()),
            _ => None,
        });
        let out = out.unwrap_or_else(|| panic!("client {index} was told of the knockout"));
        assert_eq!(out.lap, 1);
        assert_eq!(out.entity_id, slow_entity, "the slower driver is the one knocked out");
    }
}
