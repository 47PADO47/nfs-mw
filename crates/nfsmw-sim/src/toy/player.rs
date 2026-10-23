//! A seated player in the toy world: the car and the per-player race and pursuit counters.

use blackbox_net_core::session::PlayerId;

use super::car::Car;
use crate::Tick;
use crate::hash::StateHasher;

/// A player's progress round the circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RaceProgress {
    /// Index of the next checkpoint to reach.
    pub next: u8,
    /// Laps completed.
    pub laps: u8,
    /// The tick the race started for this player (the tick of joining).
    pub start: Tick,
    /// The tick the current lap started.
    pub lap_start: Tick,
    pub finished: bool,
}

/// A player's pursuit state against the police cars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChaseState {
    pub active: bool,
    /// Ticks spent nearly stopped next to a police car.
    pub bust_ticks: u16,
    /// Ticks spent far from every police car while chased.
    pub away_ticks: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSlot {
    pub id: PlayerId,
    pub car_type: u32,
    pub car: Car,
    pub race: RaceProgress,
    pub chase: ChaseState,
}

impl PlayerSlot {
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u8(self.id.0);
        h.write_u32(self.car_type);
        self.car.hash_into(h);
        h.write_u8(self.race.next);
        h.write_u8(self.race.laps);
        h.write_u32(self.race.start);
        h.write_u32(self.race.lap_start);
        h.write_bool(self.race.finished);
        h.write_bool(self.chase.active);
        h.write_u16(self.chase.bust_ticks);
        h.write_u16(self.chase.away_ticks);
    }
}

/// Where the grid slot `slot` parks: four cars a row, six metres apart, rows eight metres apart, facing +z.
pub fn grid_position(slot: usize) -> [f32; 2] {
    let column = (slot % 4) as f32;
    let row = (slot / 4) as f32;
    [(column - 1.5) * 6.0, -10.0 - row * 8.0]
}

/// A tick count as milliseconds, saturating.
pub fn ticks_to_ms(ticks: u32) -> u32 {
    let ms = u64::from(ticks) * 1000 / u64::from(crate::TICK_HZ);
    u32::try_from(ms).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_slots_never_overlap() {
        for a in 0..16 {
            for b in (a + 1)..16 {
                let (pa, pb) = (grid_position(a), grid_position(b));
                let d = ((pa[0] - pb[0]).powi(2) + (pa[1] - pb[1]).powi(2)).sqrt();
                assert!(d >= 6.0, "{a} and {b}: {d}");
            }
        }
    }

    #[test]
    fn milliseconds_from_ticks() {
        assert_eq!(ticks_to_ms(60), 1000);
        assert_eq!(ticks_to_ms(90), 1500);
        assert_eq!(ticks_to_ms(0), 0);
        assert_eq!(ticks_to_ms(u32::MAX), u32::MAX);
    }
}
