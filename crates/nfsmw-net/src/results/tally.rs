//! Session stats counted from the mode inputs: top speed, damage, busted count, pursuit time and resets.

use nfsmw_sim::{PlayerId, SimEvent, TICK_HZ};

use crate::message::MAX_SLOTS;
use crate::modes::ModeInput;

/// Milliseconds in `ticks` 60 Hz ticks, rounded down and saturated at `u32::MAX`.
pub fn ticks_to_ms(ticks: u32) -> u32 {
    let ms = u64::from(ticks) * 1000 / u64::from(TICK_HZ);
    u32::try_from(ms).unwrap_or(u32::MAX)
}

/// One player's session stats.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stats {
    pub player: PlayerId,
    /// The highest speed seen, m/s.
    pub top_speed: f32,
    /// The highest damage seen, 0..1.
    pub damage: f32,
    pub busted: u16,
    /// Ticks spent as a pursuit target.
    pub pursuit_ticks: u32,
    /// Resets the server granted.
    pub resets: u16,
    in_pursuit: bool,
}

impl Stats {
    /// The pursuit time in milliseconds.
    pub fn pursuit_ms(&self) -> u32 {
        ticks_to_ms(self.pursuit_ticks)
    }

    /// Whether the player is a pursuit target now.
    pub fn in_pursuit(&self) -> bool {
        self.in_pursuit
    }
}

/// The stats of every player seen, in player-id order, at most [`MAX_SLOTS`] of them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tally {
    rows: Vec<Stats>,
}

impl Tally {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one input. Feed every input the modes see, then call [`Tally::end_tick`] once per tick.
    pub fn feed(&mut self, input: &ModeInput) {
        match *input {
            ModeInput::Car(sample) => {
                let speed = sample.speed();
                let damage = sample.damage.clamp(0.0, 1.0);
                if let Some(row) = self.row(sample.player) {
                    row.top_speed = row.top_speed.max(speed);
                    row.damage = row.damage.max(damage);
                }
            }
            ModeInput::Sim(SimEvent::Busted { player }) => {
                if let Some(row) = self.row(player) {
                    row.busted = row.busted.saturating_add(1);
                }
            }
            ModeInput::Sim(SimEvent::PursuitStarted { player }) => {
                if let Some(row) = self.row(player) {
                    row.in_pursuit = true;
                }
            }
            ModeInput::Sim(SimEvent::PursuitEnded { player }) | ModeInput::Left { player } => {
                if let Some(row) = self.row(player) {
                    row.in_pursuit = false;
                }
            }
            ModeInput::Reset { player, .. } => {
                if let Some(row) = self.row(player) {
                    row.resets = row.resets.saturating_add(1);
                }
            }
            _ => {}
        }
    }

    /// Adds one tick of pursuit time to every player who is a pursuit target.
    pub fn end_tick(&mut self) {
        for row in self.rows.iter_mut().filter(|row| row.in_pursuit) {
            row.pursuit_ticks = row.pursuit_ticks.saturating_add(1);
        }
    }

    /// The stats of `player`, if the tally has seen them.
    pub fn get(&self, player: PlayerId) -> Option<&Stats> {
        let index = self.rows.binary_search_by_key(&player, |row| row.player).ok()?;
        Some(&self.rows[index])
    }

    /// Every player's stats, in player-id order.
    pub fn rows(&self) -> &[Stats] {
        &self.rows
    }

    /// The row of `player`, created on first sight; `None` once [`MAX_SLOTS`] players are counted.
    fn row(&mut self, player: PlayerId) -> Option<&mut Stats> {
        let index = match self.rows.binary_search_by_key(&player, |row| row.player) {
            Ok(index) => return Some(&mut self.rows[index]),
            Err(index) => index,
        };
        if self.rows.len() >= MAX_SLOTS {
            return None;
        }
        self.rows.insert(index, Stats { player, ..Stats::default() });
        Some(&mut self.rows[index])
    }
}
