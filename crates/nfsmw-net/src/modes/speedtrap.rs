//! MP-05 speedtrap: a sprint with speed traps. A pass pays only when it beats the trap's threshold and the racer's
//! best at that trap; points are the sum of the stored trap speeds in m/s (`blackbox_race::SpeedTraps`, race-mode.md
//! section 9). The largest sum ranks first, the finish time breaks a tie.
//!
//! Choices **[ours]**: no time limit and no overtime penalty in a LAN speedtrap (the original's penalty unit is
//! unconfirmed); a racer who left or was busted keeps the points but ranks behind every racer still in; passes after
//! the finish do not count.

use blackbox_race::SpeedTraps;
use nfsmw_sim::{PlayerId, Tick};

use crate::results::Placing;

use super::ModeRules;
use super::core::RaceCore;
use super::event::ModeEvent;
use super::input::ModeInput;

/// A speedtrap race.
#[derive(Debug, Clone, PartialEq)]
pub struct Speedtrap {
    core: RaceCore,
    thresholds_kmh: Vec<f32>,
    /// One per racer, in the core's order.
    traps: Vec<SpeedTraps>,
    police: bool,
}

impl Speedtrap {
    /// A speedtrap race for `players` (sorted, unique) with one threshold per trap, km/h.
    pub fn new(thresholds_kmh: &[f32], players: &[PlayerId], start_tick: Tick, police: bool) -> Self {
        let core = RaceCore::new(players, start_tick);
        let traps = players.iter().map(|_| SpeedTraps::new(thresholds_kmh.len())).collect();
        Self { core, thresholds_kmh: thresholds_kmh.to_vec(), traps, police }
    }

    fn pass(&mut self, player: PlayerId, trap: u8, kmh: f32, out: &mut Vec<ModeEvent>) {
        let threshold = match self.thresholds_kmh.get(usize::from(trap)) {
            Some(threshold) => *threshold,
            None => return,
        };
        if !kmh.is_finite() {
            return;
        }
        let index = match self.core.racers.binary_search_by_key(&player, |racer| racer.player) {
            Ok(index) if self.core.racers[index].racing() => index,
            _ => return,
        };
        let speed = match self.traps[index].record_pass(usize::from(trap), kmh, threshold) {
            Some(speed) => speed,
            None => return,
        };
        let total = self.traps[index].total();
        self.core.racers[index].points = total;
        out.push(ModeEvent::TrapScored { player, trap, speed, total });
    }
}

impl ModeRules for Speedtrap {
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if let ModeInput::TrapPass { player, trap, kmh } = *input
            && self.core.live(tick)
        {
            self.pass(player, trap, kmh, out);
        }
        self.core.feed(tick, input, out);
        self.core.check_over(out);
    }

    fn is_over(&self) -> bool {
        self.core.over
    }

    fn placings(&self) -> Vec<Placing> {
        self.core.placings(true)
    }

    fn police(&self) -> bool {
        self.police
    }
}
