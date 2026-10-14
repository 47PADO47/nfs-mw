//! The busted meter of the chase, as the world feeds it: the nearest cop on the player's height, the speed under which
//! the heat busts the player, and the timer that ends the pursuit. The timer rules are in
//! `blackbox_pursuit::busted`; this file reads the world and applies the outcome. Child of `police`, so it reads the
//! police state directly. Spec: `docs/specs/ai-pursuit.md` sections 4.4 and 5.

use super::*;
use blackbox_pursuit::busted::{BustInput, BustTuning, height_matches};

/// The busted timer's constants (`docs/specs/ai-pursuit.md` section 5.1, decomp): the timeout (`kBustedTimeout`), the
/// cop reach (`kBustedCopDistance`) and the height gate (1.5 m). The fill is +1 per second and the drain -2 per second,
/// and flashing fills four times as fast with a 90 m reach.
pub const BUST_TUNING: BustTuning = BustTuning {
    timeout: 5.0,
    reach: 15.0,
    flashing_reach: 90.0,
    height_gate: 1.5,
    fill_rate: 1.0,
    drain_rate: 2.0,
    flashing_multiplier: 4.0,
    flashing_speed_gate: false,
};

/// The speed under which the player is busted, per heat level 1 to 10, in km/h (section 5.1, the `BustSpeed` table).
const BUST_SPEED_KPH: [f32; 10] = [9.0, 12.0, 19.0, 20.0, 25.0, 25.0, 19.0, 5.0, 5.0, 5.0];

/// Kilometres per hour in one metre per second.
const KPH_PER_MPS: f32 = 3.6;

/// The speed under which the player counts as slow at `level` (clamped to 1..=10), in metres per second.
pub fn bust_speed(level: u32) -> f32 {
    BUST_SPEED_KPH[(level as usize).clamp(1, BUST_SPEED_KPH.len()) - 1] / KPH_PER_MPS
}

impl Police {
    /// Advances the busted timer one frame. Returns `true` on the frame the player is busted; the world then ends the
    /// pursuit. Nothing runs without a chase on.
    pub(super) fn tick_busted(&mut self, dt: f32, pursuing: bool, input: BustInput) -> bool {
        if !pursuing || self.heat.is_none() {
            return false;
        }
        self.busted.update(dt, &input, &BUST_TUNING)
    }

    /// The busted meter, 0 to 1 (1 once busted); it keeps its value after the chase ends, until the next chase.
    pub fn busted_meter(&self) -> f32 {
        self.busted.meter(&BUST_TUNING)
    }

    /// Whether the player has been busted in the chase (until the next chase starts). The arrest scene and the
    /// bust rewards read this.
    #[allow(dead_code)] // read by the arrest scene (E8f) and FairBusts (E12), not built yet
    pub fn is_busted(&self) -> bool {
        self.busted.is_busted()
    }
}

impl super::super::TrafficWorld {
    /// The horizontal distance to the nearest cop whose height matches the player's (`BUST_TUNING.height_gate`);
    /// infinite with none. A cop on a bridge above or a road below does not count.
    pub(super) fn nearest_matched_cop(&self, at: Vec3) -> f32 {
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, BUST_TUNING.height_gate))
            .map(|car| horizontal(car.avoidable().position - at).length())
            .fold(f32::INFINITY, f32::min)
    }

    /// The position (physics space) of the nearest cop whose height matches the player's, the one
    /// `nearest_matched_cop` measures; `None` with none. The pursuit camera frames it.
    pub fn nearest_matched_cop_position(&self, at: Vec3) -> Option<Vec3> {
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, BUST_TUNING.height_gate))
            .map(|car| car.avoidable().position)
            .min_by(|a, b| horizontal(*a - at).length().total_cmp(&horizontal(*b - at).length()))
    }

    /// The busted meter for the HUD, 0 to 1.
    pub fn busted_meter(&self) -> f32 {
        self.police.busted_meter()
    }

    /// Whether the player has been busted in the current or last chase.
    #[allow(dead_code)] // read by the arrest scene (E8f) and FairBusts (E12), not built yet
    pub fn is_busted(&self) -> bool {
        self.police.is_busted()
    }
}
