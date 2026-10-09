//! Noticing a car that presses the gas and does not move.
//! Spec: `docs/specs/ai-driver-control.md` (§7.1).

use glam::Vec3;

/// Seconds of pushing before the distance is checked.
const WINDOW: f32 = 3.0;
/// Less than this far from the anchor after the window means stuck, metres.
const MIN_TRAVEL: f32 = 3.0;

#[derive(Debug, Clone, Default)]
pub struct StuckDetector {
    anchor: Vec3,
    timer: f32,
}

impl StuckDetector {
    pub fn reset(&mut self) {
        self.timer = 0.0;
    }

    /// Call every think. Returns whether the car is stuck.
    pub fn update(
        &mut self,
        dt: f32,
        pressing_gas: bool,
        reverse_override: bool,
        staging: bool,
        position: Vec3,
    ) -> bool {
        if reverse_override || !pressing_gas {
            self.timer = 0.0;
            return false;
        }
        if staging || self.timer <= 0.0 {
            self.anchor = position;
            self.timer = dt;
            return false;
        }
        self.timer += dt;
        if self.timer < WINDOW {
            return false;
        }
        self.timer = 0.0;
        self.anchor.distance(position) < MIN_TRAVEL
    }
}
