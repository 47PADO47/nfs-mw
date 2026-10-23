//! MP-32 catch-up: a boost for trailing racers, decided by the server and shown by clients. Off unless the session
//! turns it on. Each racer behind the leader runs a `blackbox_race::CatchUp` controller over its gap to the leader,
//! sampled once a second; the controller's positive skill offset (0..1) times [`MAX_BOOST`] is the racer's boost, an
//! extra fraction of engine power the world applies. The leader and racers ahead of their own average get no boost;
//! nobody is slowed down.
//!
//! Values **[ours]**: [`MAX_BOOST`] 10 % (the docs give none); the error uses the leader's progress in place of the
//! original's "player" and a performance ratio of 1 (every car is a human's); the `basicrace` gains.

use blackbox_race::{CatchUp, CatchUpGains};
use nfsmw_sim::{PlayerId, TICK_HZ, Tick};

/// Ticks between two samples of the controller: one second.
pub const CATCH_UP_SAMPLE_TICKS: u32 = TICK_HZ;
/// The largest boost, as a fraction of engine power.
pub const MAX_BOOST: f32 = 0.10;

/// Progress through the whole race in percent: `laps_done` laps of `laps_total` plus `lap_percent` of the current
/// lap. A sprint is one lap. Clamped to 0..=100.
pub fn race_percent(laps_done: u32, lap_percent: f32, laps_total: u32) -> f32 {
    let total = laps_total.max(1) as f32;
    let lap = match lap_percent.is_finite() {
        true => lap_percent.clamp(0.0, 100.0),
        false => 0.0,
    };
    ((laps_done as f32 * 100.0 + lap) / total).clamp(0.0, 100.0)
}

#[derive(Debug, Clone, PartialEq)]
struct Trailing {
    player: PlayerId,
    controller: CatchUp,
    boost: f32,
}

/// The catch-up state of a race.
#[derive(Debug, Clone, PartialEq)]
pub struct CatchUpRule {
    enabled: bool,
    route_length: f32,
    gains: CatchUpGains,
    /// In player-id order.
    racers: Vec<Trailing>,
    next_sample: Tick,
}

impl CatchUpRule {
    /// The rule for a race of `route_length` metres (all laps), on when `enabled`.
    pub fn new(enabled: bool, route_length: f32, start_tick: Tick) -> Self {
        Self {
            enabled,
            route_length: route_length.max(0.0),
            gains: CatchUpGains::default(),
            racers: Vec::new(),
            next_sample: start_tick,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Feeds the progress of every racer still racing (`race_percent`), once per tick. On a sample tick it updates the
    /// controllers and returns `true`. A racer missing from `progress` (finished, out) loses its boost and its history.
    pub fn update(&mut self, tick: Tick, progress: &[(PlayerId, f32)]) -> bool {
        if !self.enabled || tick < self.next_sample {
            return false;
        }
        self.next_sample = tick.saturating_add(CATCH_UP_SAMPLE_TICKS);
        let leader = progress.iter().map(|(_, percent)| *percent).filter(|p| p.is_finite()).fold(0.0f32, f32::max);
        self.racers.retain(|racer| progress.iter().any(|(player, _)| *player == racer.player));
        for (player, percent) in progress {
            let percent = match percent.is_finite() {
                true => *percent,
                false => 0.0,
            };
            let error = CatchUp::error(self.route_length, leader, percent, 1.0);
            let racer = self.racer(*player);
            let skill = racer.controller.update(error, 1.0);
            racer.boost = skill.max(0.0) * MAX_BOOST;
        }
        true
    }

    /// The boost of `player`, 0 when the rule is off or the player is not trailing.
    pub fn boost(&self, player: PlayerId) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        self.racers.iter().find(|racer| racer.player == player).map_or(0.0, |racer| racer.boost)
    }

    fn racer(&mut self, player: PlayerId) -> &mut Trailing {
        let index = match self.racers.binary_search_by_key(&player, |racer| racer.player) {
            Ok(index) => index,
            Err(index) => {
                let fresh = Trailing { player, controller: CatchUp::new(self.gains), boost: 0.0 };
                self.racers.insert(index, fresh);
                index
            }
        };
        &mut self.racers[index]
    }
}
