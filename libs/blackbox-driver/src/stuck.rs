//! Noticing a car that presses the gas and does not move.
//! Spec: `docs/specs/ai-driver-control.md` (§7.1).

use glam::Vec3;

/// Seconds of pushing before the distance is checked.
const WINDOW: f32 = 3.0;
/// Less than this far from the anchor after the window means stuck, metres.
const MIN_TRAVEL: f32 = 3.0;

/// **[ours]** The window also asks for this much travel along the car's own heading (the integral of the forward
/// speed), metres. The rewrite's wall contact lets a car pushing nose-in slide sideways along the wall, which moves it
/// past the anchor test while it makes no headway; the original's wedged cars stay put, so the original's test alone
/// catches them (spec section 7.1).
const MIN_PROGRESS: f32 = 3.0;

#[derive(Debug, Clone, Default)]
pub struct StuckDetector {
    anchor: Vec3,
    timer: f32,
    /// Metres travelled along the heading since the anchor.
    progress: f32,
}

impl StuckDetector {
    /// Visits the anchor and elapsed time by their raw float bits.
    pub fn visit_state(&self, visit: &mut impl FnMut(u32)) {
        for value in self.anchor.to_array().into_iter().chain([self.timer, self.progress]) {
            visit(value.to_bits());
        }
    }
    pub fn reset(&mut self) {
        self.timer = 0.0;
    }

    /// Call every think with the car's position and signed forward speed (m/s). Returns whether the car is stuck.
    pub fn update(
        &mut self,
        dt: f32,
        pressing_gas: bool,
        reverse_override: bool,
        staging: bool,
        position: Vec3,
        forward_speed: f32,
    ) -> bool {
        if reverse_override || !pressing_gas {
            self.timer = 0.0;
            return false;
        }
        if staging || self.timer <= 0.0 {
            self.anchor = position;
            self.timer = dt;
            self.progress = 0.0;
            return false;
        }
        self.timer += dt;
        self.progress += forward_speed.abs() * dt;
        if self.timer < WINDOW {
            return false;
        }
        self.timer = 0.0;
        self.anchor.distance(position) < MIN_TRAVEL || self.progress < MIN_PROGRESS
    }
}
