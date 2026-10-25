//! The race scenarios: a three-lap circuit of six racers, eight gates a lap, on its grid, under way and finished.

use nfsmw_data::gameplay::RaceType;

use super::common::{HEADING, at, car, on_ground};
use crate::hud::{
    Count, HudEvent, HudState, LeaderEntry, LeaderRow, Leaders, MapBlip, RACER, RaceHud, RaceInfo, RivalGap,
};

/// The racers, the player among them.
const RACERS: u32 = 6;
/// The laps, and the gates of each (seven checkpoints and the finish line).
const LAPS: u32 = 3;
const GATES_PER_LAP: u32 = 8;
/// The length of the whole race, metres.
const LENGTH_M: f32 = 12_600.0;
/// The name of the racer the player is measured against: a fixture label, not a game name.
const RIVAL: &str = "Racer 1";

/// The grid: 6th of six, the countdown at three, the engine held at 4500 rpm.
pub fn intro(viewport: [f32; 2]) -> HudState {
    let race = RaceHud {
        place: RACERS,
        racers: RACERS,
        lap: Some((1, LAPS)),
        clock: 0.0,
        progress: 0.0,
        count: Some(Count::Three),
    };
    let info = RaceInfo {
        kind: RaceType::Circuit,
        next_gate: Some(on_ground(180.0, 0.0)),
        gate: Some(1),
        gates: LAPS * GATES_PER_LAP,
        remaining_m: Some(LENGTH_M),
        rival_gap: None,
    };
    HudState {
        race: Some(race),
        leaders: Some(board(RACERS, [0.0; 6])),
        race_info: Some(info),
        ..car(0.0, 1, 4500.0, viewport)
    }
}

/// Lap 2 of 3 at 187 km/h: 2nd of six, 55 % of the race done, 4.2 km left, gate 15 of 24 (checkpoint 7 of the lap)
/// next, the leader 1.42 s ahead at the last checkpoint both passed. The player has just passed checkpoint 7 and moved up
/// from 3rd.
pub fn racing(viewport: [f32; 2]) -> HudState {
    let race = RaceHud { place: 2, racers: RACERS, lap: Some((2, LAPS)), clock: 151.6, progress: 0.55, count: None };
    let info = RaceInfo {
        kind: RaceType::Circuit,
        next_gate: Some(on_ground(240.0, -12.0)),
        gate: Some(GATES_PER_LAP + 7),
        gates: LAPS * GATES_PER_LAP,
        remaining_m: Some(4200.0),
        rival_gap: Some(RivalGap { seconds: 1.42, ahead: false, name: RIVAL.to_owned() }),
    };
    let rival = MapBlip { style: RACER, position: at(38.0, 2.5), active: false, heading: Some(HEADING) };
    HudState {
        race: Some(race),
        leaders: Some(board(2, [56.3, 55.0, 54.1, 52.8, 50.2, 47.9])),
        race_info: Some(info),
        blips: vec![rival],
        events: vec![
            HudEvent::Checkpoint { number: 7, total: GATES_PER_LAP },
            HudEvent::PositionChanged { from: 3, to: 2 },
        ],
        ..car(187.0, 5, 6900.0, viewport)
    }
}

/// Over the line in 2nd at 4:47.35, slowing in 3rd: every gate passed, the leader 0.86 s ahead at the finish, the
/// results screens next.
pub fn results(viewport: [f32; 2]) -> HudState {
    let race = RaceHud { place: 2, racers: RACERS, lap: Some((LAPS, LAPS)), clock: 287.35, progress: 1.0, count: None };
    let info = RaceInfo {
        kind: RaceType::Circuit,
        next_gate: None,
        gate: None,
        gates: LAPS * GATES_PER_LAP,
        remaining_m: None,
        rival_gap: Some(RivalGap { seconds: 0.86, ahead: false, name: RIVAL.to_owned() }),
    };
    HudState {
        race: Some(race),
        leaders: Some(board(2, [100.0, 100.0, 99.2, 98.4, 97.0, 95.5])),
        race_info: Some(info),
        ..car(96.0, 3, 3800.0, viewport)
    }
}

/// The leaderboard with the player in `player_place` and each place's share of the race done, first place first. The
/// rival names are fixture labels.
fn board(player_place: u32, percents: [f32; 6]) -> Leaders {
    let entries = percents
        .into_iter()
        .enumerate()
        .map(|(i, percent)| {
            let place = i as u32 + 1;
            let player = place == player_place;
            let name = match player {
                true => "Player".to_owned(),
                false => format!("Racer {place}"),
            };
            LeaderEntry { place, player, row: LeaderRow { name, percent, points: 0.0, out: false, busted: false } }
        })
        .collect();
    Leaders::pick(entries, LENGTH_M, false)
}
