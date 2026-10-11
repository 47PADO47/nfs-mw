//! The sun the world is lit by. Its daily motion follows `docs/specs/time-of-day.md`: a phase that
//! runs back and forth between two limits, with the sun's direction built from that phase.

use glam::Vec3;
use nfsmw_data::time_of_day::{advance_sun_phase, sun_direction, sun_phase_limit};

/// The orbit's angle offsets (the lighting object's `+0x0C` and `+0x10`), at their constructor defaults.
const ANGLE_A: f32 = 3.28;
const ANGLE_B: f32 = 0.5;
/// The phase at the start: noon. The game's phase at the start of a drive is not yet known.
const START_PHASE: f32 = 0.5;

/// The direction the sun's light travels at noon. Used where nothing updates the sun each frame.
pub(super) const DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -1.0);

/// The direction from the cars towards the sun at noon, for the rigs set up before the first frame.
pub(super) fn to_sun() -> Vec3 {
    -DIRECTION
}

/// The sun's daily motion.
pub(super) struct Sun {
    phase: f32,
    dir: i32,
    lo: f32,
}

impl Sun {
    /// The sun at the start of the day, in the free-roam phase limit.
    pub(super) fn new() -> Self {
        Self { phase: START_PHASE, dir: 1, lo: sun_phase_limit(0) }
    }

    /// Moves the sun on by `dt` seconds (the frame time).
    pub(super) fn advance(&mut self, dt: f32) {
        let (phase, dir) = advance_sun_phase(self.phase, self.dir, 1.0, dt, self.lo);
        self.phase = phase;
        self.dir = dir;
    }

    /// The direction from the world towards the sun.
    pub(super) fn to_sun(&self) -> Vec3 {
        Vec3::from_array(sun_direction(self.phase, self.lo, ANGLE_A, ANGLE_B))
    }

    /// The direction the sun's light travels: the opposite of `to_sun`.
    pub(super) fn direction(&self) -> Vec3 {
        -self.to_sun()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_starts_at_a_unit_direction_above_the_horizon() {
        let sun = Sun::new();
        let to = sun.to_sun();
        assert!((to.length() - 1.0).abs() < 1e-5);
        assert!(to.z > 0.0);
        assert_eq!(sun.direction(), -to);
    }

    #[test]
    fn advancing_moves_the_sun() {
        let mut sun = Sun::new();
        let before = sun.to_sun();
        for _ in 0..600 {
            sun.advance(1.0 / 60.0);
        }
        assert_ne!(sun.to_sun(), before);
    }
}
