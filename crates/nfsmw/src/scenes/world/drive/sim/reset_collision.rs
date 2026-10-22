//! Reset object-penetration admission; `docs/specs/reset-object-collision.md`.

use super::CarSim;

#[derive(Default)]
pub(super) struct ResetCollision {
    time: f64,
    last_refused: f64,
    remaining: f32,
}

impl ResetCollision {
    pub(super) fn advance(&mut self, dt: f32) {
        self.time += f64::from(dt);
        if self.active() && self.time > self.last_refused + 1.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }

    pub(super) fn active(&self) -> bool {
        self.remaining > 0.0
    }

    pub(super) fn admit(&mut self) -> bool {
        if self.active() {
            self.last_refused = self.time;
            return false;
        }
        true
    }
}

impl CarSim {
    /// Called only after a successful replacement placement, not initial spawning.
    pub fn begin_reset_collision_grace(&mut self) {
        self.reset_collision.remaining = 2.0;
    }

    pub fn reset_collision_active(&self) -> bool {
        self.reset_collision.active()
    }
}

#[cfg(test)]
mod tests;
