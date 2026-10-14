//! The day-night clock's curves: the hour of day, the sun's phase for it, and the hours the lights are on.
//!
//! These are this project's own curves and defaults, not values read from the install or taken from any mod. The
//! sun's phase is the orbit's `p` (see `time_of_day::sun_direction`): from sunrise to sunset it runs from the low
//! limit to the high limit, so noon is 0.5. Before sunrise it holds the low limit and after sunset the high limit.
//! The configuration keys are named after the usual day-night settings; the mapping is documented in
//! `docs/specs/time-of-day.md`.

use serde::Deserialize;

const HOURS: f32 = 24.0;

/// The clock's settings. Missing keys take the defaults.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct DayNightConfig {
    /// A fixed hour of day in `[0, 24)`. When set, the clock does not run.
    pub force_time: Option<f32>,
    /// Use the wall clock. Read, but not applied yet.
    pub real_time: bool,
    /// Start the clock at a random hour. Read, but not applied yet.
    pub random_startup_time: bool,
    /// The hour the clock starts at, when it runs.
    pub start_hour: f32,
    /// Real seconds for one game hour.
    pub seconds_per_hour: f32,
    /// The hour the sun rises. Sunrise is before sunset in the same day.
    pub sun_rise: f32,
    /// The hour the sun sets.
    pub sun_set: f32,
    /// The hour the lights come on.
    pub lights_on: f32,
    /// The hour the lights go off.
    pub lights_off: f32,
    /// The sun's first angle offset (the lighting object's `+0x0C`), in radians.
    pub sun_azimuth: f32,
}

impl Default for DayNightConfig {
    fn default() -> Self {
        Self {
            force_time: None,
            real_time: false,
            random_startup_time: false,
            start_hour: 12.0,
            seconds_per_hour: 150.0,
            sun_rise: 6.0,
            sun_set: 18.0,
            lights_on: 18.5,
            lights_off: 6.5,
            sun_azimuth: 3.28,
        }
    }
}

/// Wraps an hour into `[0, 24)`. A non-finite hour reads as midnight.
pub fn wrap_hour(hour: f32) -> f32 {
    if !hour.is_finite() {
        return 0.0;
    }
    hour.rem_euclid(HOURS)
}

/// The hour of day after `seconds` of simulated time, or the forced hour when `force_time` is set.
pub fn clock_hour(cfg: &DayNightConfig, seconds: f32) -> f32 {
    if let Some(hour) = cfg.force_time {
        return wrap_hour(hour);
    }
    wrap_hour(cfg.start_hour + seconds / cfg.seconds_per_hour.max(f32::EPSILON))
}

/// The sun's phase for the hour, between the orbit's limits `lo` and `1 - lo`. From sunrise to sunset the phase
/// runs linearly from `lo` to `1 - lo`. Before sunrise it holds `lo`, and after sunset it holds `1 - lo`. The
/// phase jumps from `1 - lo` to `lo` at midnight. A sunset that is not after sunrise holds `lo` all day.
pub fn sun_phase(hour: f32, cfg: &DayNightConfig, lo: f32) -> f32 {
    let (rise, set) = (cfg.sun_rise, cfg.sun_set);
    if set <= rise || hour < rise {
        return lo;
    }
    if hour > set {
        return 1.0 - lo;
    }
    let t = (hour - rise) / (set - rise);
    lo + (1.0 - 2.0 * lo) * t
}

/// Whether the lights are on at the hour: from `lights_on` to `lights_off`, across midnight when `lights_on` is
/// later in the day than `lights_off`.
pub fn lights_on(hour: f32, cfg: &DayNightConfig) -> bool {
    let h = wrap_hour(hour);
    if cfg.lights_on > cfg.lights_off {
        return h >= cfg.lights_on || h < cfg.lights_off;
    }
    h >= cfg.lights_on && h < cfg.lights_off
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_of_day::{dusk_factor, sun_direction};

    const LO: f32 = 0.05;

    #[test]
    fn the_clock_steps_to_night() {
        let cfg = DayNightConfig::default();
        // Seven game hours after noon is 19:00: past sunset, with the lights on.
        let hour = clock_hour(&cfg, 7.0 * cfg.seconds_per_hour);
        assert!((hour - 19.0).abs() < 1e-3, "{hour}");
        assert!(lights_on(hour, &cfg));
        assert_eq!(sun_phase(hour, &cfg, LO), 1.0 - LO);
    }

    #[test]
    fn the_night_sun_takes_the_low_sun_presets() {
        let cfg = DayNightConfig::default();
        let night = sun_phase(22.0, &cfg, LO);
        let z = sun_direction(night, LO, cfg.sun_azimuth, 0.5)[2];
        assert_eq!(dusk_factor(z), 1.0);
        let noon = sun_phase(12.0, &cfg, LO);
        let z = sun_direction(noon, LO, cfg.sun_azimuth, 0.5)[2];
        assert_eq!(dusk_factor(z), 0.0);
    }

    #[test]
    fn noon_is_the_middle_of_the_day_phase() {
        let cfg = DayNightConfig::default();
        assert!((sun_phase(12.0, &cfg, LO) - 0.5).abs() < 1e-6);
        assert_eq!(sun_phase(3.0, &cfg, LO), LO);
    }

    #[test]
    fn the_lights_come_on_at_lights_on_and_go_off_at_lights_off() {
        let cfg = DayNightConfig::default();
        assert!(lights_on(cfg.lights_on, &cfg));
        assert!(!lights_on(cfg.lights_off, &cfg));
        assert!(!lights_on(12.0, &cfg));
        assert!(lights_on(0.0, &cfg));
    }

    #[test]
    fn a_forced_time_ignores_the_clock() {
        let cfg = DayNightConfig { force_time: Some(3.0), ..DayNightConfig::default() };
        assert_eq!(clock_hour(&cfg, 99_999.0), 3.0);
    }

    #[test]
    fn the_clock_wraps_past_midnight() {
        let cfg = DayNightConfig { start_hour: 23.0, seconds_per_hour: 1.0, ..DayNightConfig::default() };
        assert!((clock_hour(&cfg, 2.0) - 1.0).abs() < 1e-4);
    }
}
