//! What the HUD knows of the race beyond the original package's readout: the race type, the next gate, how much of the
//! route is left, and the gap to the nearest rival. The world fills it from the race engine
//! (`scenes/world/race/hud_info.rs`).
//!
//! The rival gap is the difference between the player's and the rival's passage times at the latest checkpoint both
//! have passed (`blackbox_race::Racer::checkpoint_times`). It is a magnitude with a side, never a signed number: "1.42 s
//! ahead" or "0.85 s behind". Before the first shared checkpoint there is no gap, and the drag, speed-trap and tollbooth
//! races (and the pursuit challenges) show none: their standing is not a matter of passage times.

use nfsmw_data::gameplay::RaceType;

/// The race the player runs, for the Custom HUD. `None` in `HudState` outside a race.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceInfo {
    pub kind: RaceType,
    /// The centre of the gate the player drives to next (a checkpoint or the finish line), render space (x, y, z),
    /// metres. `None` once the player is done.
    pub next_gate: Option<[f32; 3]>,
    /// The number of that gate, from 1, among `gates`. `None` once the player is done.
    pub gate: Option<u32>,
    /// The gates of the whole race: each lap's checkpoints and its finish line.
    pub gates: u32,
    /// Metres of the race's route still to drive from where the player is on it. `None` without a route (a world with
    /// no road network) or once the player is done.
    pub remaining_m: Option<f32>,
    /// The gap to the nearest rival; see the module.
    pub rival_gap: Option<RivalGap>,
}

/// The time between the player and the rival at the latest checkpoint both have passed.
#[derive(Debug, Clone, PartialEq)]
pub struct RivalGap {
    /// The size of the gap, seconds, never negative.
    pub seconds: f32,
    /// The player passed that checkpoint first (or at the same time).
    pub ahead: bool,
    /// The rival's leaderboard name.
    pub name: String,
}

/// Whether a race of `kind` shows a rival gap: not the drag, speed-trap and tollbooth races, nor the pursuit
/// challenges.
pub fn shows_rival_gap(kind: RaceType) -> bool {
    !matches!(kind, RaceType::Drag | RaceType::SpeedTrap | RaceType::Tollbooth | RaceType::Challenge)
}

/// The racer the player is measured against: the one a place ahead, or for the leader the one a place behind. `places`
/// is each racer's place (1 for first) by racer index, and `out` whether the racer is out of the race. `None` when that
/// racer is out or there is none.
pub fn rival_of(places: &[u32], player: usize, out: &[bool]) -> Option<usize> {
    let place = *places.get(player)?;
    let wanted = match place {
        1 => 2,
        _ => place - 1,
    };
    let rival = places.iter().enumerate().position(|(i, p)| i != player && *p == wanted)?;
    if out.get(rival).copied().unwrap_or(false) {
        return None;
    }
    Some(rival)
}

/// The gap at the latest checkpoint both racers passed: its size in seconds and whether the player was there first.
/// `player` and `rival` are their passage times in race order. `None` before the first shared checkpoint.
pub fn checkpoint_gap(player: &[f32], rival: &[f32]) -> Option<(f32, bool)> {
    let shared = player.len().min(rival.len());
    let last = shared.checked_sub(1)?;
    let (mine, theirs) = (player[last], rival[last]);
    Some(((mine - theirs).abs(), mine <= theirs))
}

#[cfg(test)]
#[path = "race_info_tests.rs"]
mod tests;
