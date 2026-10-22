//! A whole round of a circuit (MP-02, MP-70, MP-71): the clients steer round the toy circuit from their own blocks,
//! the race rules finish them, the results go out, the replay and the result record are written, and the session
//! returns to the lobby.

use std::path::PathBuf;

use nfsmw_net::input::InputCode;
use nfsmw_net::message::{Message, SessionPhase};
use nfsmw_net::record::Replay;
use nfsmw_sim::InputState;
use nfsmw_sim::toy::{RaceConfig, ToyConfig};

use super::client::TestClient;
use super::harness::{Harness, test_config};

/// Steers towards `target` from the client's newest own block, as the toy world's own bots do.
fn steer(client: &TestClient, target: [f32; 2]) -> InputCode {
    let Some(own) = client.snapshots.last().and_then(|snapshot| snapshot.own) else {
        return InputCode::NEUTRAL;
    };
    let [vx, _, vz] = own.linear_velocity;
    let speed = vx.hypot(vz);
    let dir = match speed > 1.0 {
        true => [vx / speed, vz / speed],
        false => [0.0, 1.0],
    };
    let (dx, dz) = (target[0] - own.position[0], target[1] - own.position[2]);
    let len = dx.hypot(dz).max(1.0e-3);
    let ahead = (dir[0] * dx + dir[1] * dz) / len;
    let left = (dir[1] * dx - dir[0] * dz) / len;
    let turn = match ahead < 0.0 {
        true => -left.signum(),
        false => (-left * 3.0).clamp(-1.0, 1.0),
    };
    let throttle = match speed > 30.0 {
        true => 0.4,
        false => 1.0,
    };
    InputCode::quantise(&InputState { throttle, steer: turn, ..InputState::default() })
}

#[test]
fn a_circuit_runs_to_its_results_and_back_to_the_lobby() {
    let dir = std::env::temp_dir().join(format!("nfsmw-server-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut config = test_config();
    for (key, value) in [("mode", "circuit"), ("laps", "1")] {
        nfsmw_net::settings::set(&mut config.settings, key, value).expect("a valid setting");
    }
    config.replay_dir = Some(dir.clone());
    let race = RaceConfig::circuit(1);
    let checkpoints = race.checkpoints.clone();
    let toy = ToyConfig { race: Some(race), ..ToyConfig::bare() };
    let mut h = Harness::new(config, toy);
    let racers: Vec<usize> = (0..2).map(|seed| h.connect(90 + seed)).collect();
    h.join_all(&racers);
    h.start_race(&racers);

    let mut next = vec![0usize; racers.len()];
    for _ in 0..4000 {
        if h.clients[racers[0]].got(|message| matches!(message, Message::RaceResults(_))) {
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
            steer(client, checkpoints[next[index] % checkpoints.len()])
        });
    }
    let results = h.clients[racers[0]].messages.iter().find_map(|message| match message {
        Message::RaceResults(results) => Some(results.clone()),
        _ => None,
    });
    let results = results.expect("the round ends with results");
    assert_eq!(results.rows.len(), 2);
    assert!(results.rows.iter().any(|row| row.place == 1 && row.time_ms > 0), "a winner with a time");
    assert!(h.clients[racers[1]].got(|message| matches!(message, Message::RaceFinish(_))), "finish events");

    h.until("the lobby again", 600, InputCode::NEUTRAL, |h| {
        h.clients[racers[0]]
            .got(|message| matches!(message, Message::PhaseChange(change) if change.phase == SessionPhase::Lobby))
    });
    h.server.shutdown();
    let files: Vec<PathBuf> =
        std::fs::read_dir(&dir).expect("the replay dir").map(|entry| entry.expect("entry").path()).collect();
    let replay = files.iter().find(|path| path.extension().is_some_and(|ext| ext == "nfsrpl")).expect("a replay");
    let bytes = std::fs::read(replay).expect("readable");
    let replay = Replay::open(&bytes).expect("a valid replay");
    assert!(replay.has_index(), "finished with an index");
    assert!(replay.frames().count() > 100, "snapshots and events recorded");
    assert!(files.iter().any(|path| path.extension().is_some_and(|ext| ext == "nmrr")), "the result record");
    let _ = std::fs::remove_dir_all(&dir);
}
