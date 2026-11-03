//! The sun the world is lit by. Its daily motion follows `docs/specs/time-of-day.md`: the phase is set from the
//! day-night clock (`nfsmw_data::day_night::sun_phase`), and the sun's direction is built from that phase.

use glam::Vec3;
use nfsmw_data::day_night::DayNightConfig;
use nfsmw_data::time_of_day::{sun_direction, sun_phase_limit};

use super::full_cycle::{self, Celestial};

/// The orbit's second angle offset (the lighting object's `+0x10`), at its constructor default. The first one
/// (`+0x0C`) is the configured `sun_azimuth`.
pub(super) const ANGLE_B: f32 = 0.5;
/// The phase at the start: noon. The game's phase at the start of a drive is not yet known.
const START_PHASE: f32 = 0.5;

/// The direction the sun's light travels at noon. Used where nothing updates the sun each frame.
pub(super) const DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -1.0);

/// The direction from the cars towards the sun at noon, for the rigs set up before the first frame.
pub(super) fn to_sun() -> Vec3 {
    -DIRECTION
}

/// The sun's position on its orbit, or the optional full day/night cycle when that is on.
pub(super) struct Sun {
    phase: f32,
    lo: f32,
    azimuth: f32,
    /// The cycle, evaluated once per clock hour; every consumer reads this one value.
    cycle: Option<Celestial>,
}

impl Sun {
    /// The sun at noon, in the free-roam phase limit, with the default azimuth.
    /// TODO: take the configured `sun_azimuth` once the day-night config is loaded from the settings file.
    pub(super) fn new() -> Self {
        Self { phase: START_PHASE, lo: sun_phase_limit(0), azimuth: DayNightConfig::default().sun_azimuth, cycle: None }
    }

    /// Sets the phase, for the time of day (see `nfsmw_data::day_night::sun_phase`).
    pub(super) fn set_phase(&mut self, phase: f32) {
        self.phase = phase;
    }

    /// Drive the world from the full day/night cycle at `hour`, or from the orbit when `None`. The cycle is
    /// re-evaluated only when the hour changes.
    pub(super) fn set_cycle(&mut self, hour: Option<f32>) {
        let Some(hour) = hour else {
            self.cycle = None;
            return;
        };
        if self.cycle.is_some_and(|c| c.hour == nfsmw_data::day_night::wrap_hour(hour)) {
            return;
        }
        self.cycle = Some(full_cycle::at(hour));
    }

    /// The full day/night cycle this frame, when it is on.
    pub(super) fn cycle(&self) -> Option<Celestial> {
        self.cycle
    }

    /// The direction from the world towards the current directional light (the moon's at night in the cycle).
    pub(super) fn to_sun(&self) -> Vec3 {
        match self.cycle {
            Some(c) => c.light,
            None => Vec3::from_array(sun_direction(self.phase, self.lo, self.azimuth, ANGLE_B)),
        }
    }

    /// The direction towards the sun as drawn in the sky: its disk, its glare and the sky's scattering.
    pub(super) fn visible(&self) -> Vec3 {
        self.cycle.map_or_else(|| self.to_sun(), |c| c.sun)
    }

    /// The sun height the time-of-day presets are read at. In the cycle it never reads brighter than dusk at night.
    pub(super) fn preset_height(&self) -> f32 {
        self.cycle.map_or_else(|| self.to_sun().z, |c| c.preset_height)
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
    fn the_cycle_separates_the_drawn_sun_the_light_and_the_preset_height() {
        let mut sun = Sun::new();
        let orbit = sun.to_sun();
        assert_eq!((sun.visible(), sun.preset_height()), (orbit, orbit.z), "without the cycle all three agree");
        sun.set_cycle(Some(0.0));
        let c = full_cycle::at(0.0);
        assert_eq!((sun.to_sun(), sun.visible(), sun.preset_height()), (c.light, c.sun, c.preset_height));
        sun.set_cycle(Some(24.0));
        assert_eq!(sun.cycle(), Some(c), "a wrapped hour reuses the evaluation");
        sun.set_cycle(None);
        assert_eq!(sun.to_sun(), orbit);
    }
}
