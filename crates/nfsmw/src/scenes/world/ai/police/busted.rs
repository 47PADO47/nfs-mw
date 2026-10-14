//! The busted meter of the chase, as the world feeds it: the nearest cop on the player's height, the speed under which
//! the heat busts the player, and the timer that ends the pursuit. The timer rules are in `blackbox_pursuit::busted`;
//! this file reads the world and applies the outcome. The constants come from the Bartender file (the bust rules) and
//! the FairBusts file (the reset penalty), see `crate::settings`. Child of `police`, so it reads the police state
//! directly. Spec: `docs/specs/ai-pursuit.md` sections 4.4 and 5, `docs/specs/fairbusts-reset.md`.

use super::*;
use blackbox_pursuit::busted::{BustInput, BustTuning, height_matches};

/// Seconds of the post-reset blink after a manual reset: the player flashes (the reset penalty applies) while it runs.
/// A reset sets the invulnerability timer to 2 s (`docs/specs/vehicle-rigid-body.md` section 7).
pub const RESET_BLINK: f32 = 2.0;

/// The busted timer's fill and drain rates, per second: net +1 filling and -2 draining (`docs/specs/ai-pursuit.md`
/// section 5.1). Not configurable.
const FILL_RATE: f32 = 1.0;
const DRAIN_RATE: f32 = 2.0;

impl Police {
    /// The busted timer's constants: the bust rules from the Bartender file and the reset penalty from FairBusts.
    pub fn bust_tuning(&self) -> BustTuning {
        let rules = self.settings().busted;
        let penalty = self.reset_penalty();
        BustTuning {
            timeout: rules.timeout,
            reach: rules.reach,
            flashing_reach: penalty.reach,
            height_gate: rules.height_gate,
            fill_rate: FILL_RATE,
            drain_rate: DRAIN_RATE,
            flashing_multiplier: penalty.multiplier,
            flashing_speed_gate: penalty.speed_gate,
        }
    }

    /// The speed under which the player is busted at `level`, metres per second.
    pub fn bust_speed(&self, level: u32) -> f32 {
        self.settings().bust_speed_mps(level)
    }

    /// Advances the busted timer one frame. Returns `true` on the frame the player is busted; the world then ends the
    /// pursuit. Nothing runs without a chase on.
    pub(super) fn tick_busted(&mut self, dt: f32, pursuing: bool, input: BustInput) -> bool {
        if !pursuing || self.heat.is_none() {
            return false;
        }
        let tuning = self.bust_tuning();
        self.busted.update(dt, &input, &tuning)
    }

    /// The busted meter, 0 to 1 (1 once busted); it keeps its value after the chase ends, until the next chase.
    pub fn busted_meter(&self) -> f32 {
        self.busted.meter(&self.bust_tuning())
    }

    /// A manual reset (the player's reset key) starts the post-reset blink.
    pub fn note_manual_reset(&mut self) {
        self.reset_blink = RESET_BLINK;
    }

    /// Whether the player is flashing: the post-reset blink is on.
    pub fn flashing(&self) -> bool {
        self.reset_blink > 0.0
    }

    /// Whether the player has been busted in the chase (until the next chase starts). The arrest scene and the
    /// bust rewards read this.
    #[allow(dead_code)] // read by the arrest scene (E8f), not built yet
    pub fn is_busted(&self) -> bool {
        self.busted.is_busted()
    }
}

impl super::super::TrafficWorld {
    /// The horizontal distance to the nearest cop whose height matches the player's (`height_gate`); infinite with
    /// none. A cop on a bridge above or a road below does not count.
    pub(super) fn nearest_matched_cop(&self, at: Vec3) -> f32 {
        let gate = self.police.bust_tuning().height_gate;
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, gate))
            .map(|car| horizontal(car.avoidable().position - at).length())
            .fold(f32::INFINITY, f32::min)
    }

    /// The position (physics space) of the nearest cop whose height matches the player's, the one
    /// `nearest_matched_cop` measures; `None` with none. The pursuit camera frames it.
    pub fn nearest_matched_cop_position(&self, at: Vec3) -> Option<Vec3> {
        let gate = self.police.bust_tuning().height_gate;
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, gate))
            .map(|car| car.avoidable().position)
            .min_by(|a, b| horizontal(*a - at).length().total_cmp(&horizontal(*b - at).length()))
    }

    /// The busted meter for the HUD, 0 to 1.
    pub fn busted_meter(&self) -> f32 {
        self.police.busted_meter()
    }

    /// Whether the player has been busted in the current or last chase.
    #[allow(dead_code)] // read by the arrest scene (E8f), not built yet
    pub fn is_busted(&self) -> bool {
        self.police.is_busted()
    }
}
