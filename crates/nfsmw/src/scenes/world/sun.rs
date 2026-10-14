//! The sun the world is lit by. Its daily motion follows `docs/specs/time-of-day.md`: the phase is set from the
//! day-night clock (`nfsmw_data::day_night::sun_phase`), and the sun's direction is built from that phase.

use glam::Vec3;
use nfsmw_data::day_night::DayNightConfig;
use nfsmw_data::time_of_day::{advance_sun_phase, sun_direction, sun_phase_limit};

/// The orbit's second angle offset (the lighting object's `+0x10`), at its constructor default. The first one
/// (`+0x0C`) is the configured `sun_azimuth`.
const ANGLE_B: f32 = 0.5;
/// The phase at the start: noon. The game's phase at the start of a drive is not yet known.
const START_PHASE: f32 = 0.5;

/// The direction the sun's light travels at noon. Used where nothing updates the sun each frame.
pub(super) const DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -1.0);

/// The direction from the cars towards the sun at noon, for the rigs set up before the first frame.
pub(super) fn to_sun() -> Vec3 {
    -DIRECTION
}

/// The sun's position on its orbit.
pub(super) struct Sun {
    phase: f32,
    lo: f32,
    dir: i32,
    azimuth: f32,
}

impl Sun {
    /// The sun at noon, in the free-roam phase limit, with the default azimuth.
    /// TODO: take the configured `sun_azimuth` once the day-night config is loaded from the settings file.
    pub(super) fn new() -> Self {
        Self { phase: START_PHASE, lo: sun_phase_limit(0), dir: 1, azimuth: DayNightConfig::default().sun_azimuth }
    }

    /// Diagnostic freeze holds lighting, without stopping traffic or accruing a catch-up step.
    pub(super) fn update(&mut self, dt: f32, frozen: bool) {
        if frozen {
            return;
        }
        self.advance(dt);
    }

    /// Moves the sun on by `dt` seconds (the frame time).
    pub(super) fn advance(&mut self, dt: f32) {
        let (phase, dir) = advance_sun_phase(self.phase, self.dir, 1.0, dt, self.lo);
        self.phase = phase;
        self.dir = dir;
    }
    /// Sets the phase, for the time of day (see `nfsmw_data::day_night::sun_phase`).
    pub(super) fn set_phase(&mut self, phase: f32) {
        self.phase = phase;
    }

    /// The direction from the world towards the sun.
    pub(super) fn to_sun(&self) -> Vec3 {
        Vec3::from_array(sun_direction(self.phase, self.lo, self.azimuth, ANGLE_B))
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
    fn setting_the_phase_moves_the_sun() {
        let mut sun = Sun::new();
        let before = sun.to_sun();
        sun.set_phase(0.1);
        assert_ne!(sun.to_sun(), before);
    }

    #[test]
    fn freezing_holds_lighting_and_resume_does_not_catch_up() {
        let mut sun = Sun::new();
        let before = sun.to_sun();
        sun.update(600.0, true);
        assert_eq!(sun.to_sun(), before);
        sun.update(1.0, false);
        let mut expected = Sun::new();
        expected.advance(1.0);
        assert_eq!(sun.to_sun(), expected.to_sun());
    }
}
