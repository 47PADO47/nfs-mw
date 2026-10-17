//! The world's day-night clock. It counts the simulated time, sets the sun for the hour of day, and switches the
//! player's headlights at the clock's lights-on and lights-off hours. The curves are in `nfsmw_data::day_night`.
//! Not yet done: the sky colour (the sky model, a later epic) and the world's own lights.

use nfsmw_data::day_night::{self, DayNightConfig};
use nfsmw_data::time_of_day::sun_phase_limit;

use super::WorldScene;

/// The clock's state.
pub(super) struct DayClock {
    cfg: DayNightConfig,
    /// Simulated seconds since the world started.
    seconds: f64,
    sun_mode: crate::viewer::SunMode,
    phase: f64,
    direction: f64,
    /// The lights state last applied to the drive, so a manual headlight toggle holds until the next switch.
    lights: Option<bool>,
    /// A mod has held the hour (`bb.time` freeze). The clock stops as it does for the lab's freeze.
    held: bool,
}

impl DayClock {
    pub(super) fn new(cfg: DayNightConfig) -> Self {
        Self { cfg, seconds: 0.0, sun_mode: Default::default(), phase: 0.5, direction: 1.0, lights: None, held: false }
    }

    /// Whether a mod has held the hour.
    pub(super) fn held(&self) -> bool {
        self.held
    }

    pub(super) fn set_held(&mut self, held: bool) {
        self.held = held;
    }

    /// Moves the clock on by `dt` simulated seconds.
    pub(super) fn advance(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.seconds += f64::from(dt);
        if self.sun_mode == crate::viewer::SunMode::OriginalCycle {
            let lo = f64::from(sun_phase_limit(0));
            let span = 1.0 - 2.0 * lo;
            let offset = self.phase - lo;
            let unfolded = match self.direction > 0.0 {
                true => offset,
                false => 2.0 * span - offset,
            };
            let at = (unfolded + f64::from(dt) / 1200.0).rem_euclid(2.0 * span);
            self.direction = match at < span {
                true => 1.0,
                false => -1.0,
            };
            self.phase = lo
                + match at < span {
                    true => at,
                    false => 2.0 * span - at,
                };
        }
    }

    /// The lab freezes the lighting clock without accruing a catch-up step.
    pub(super) fn update(&mut self, dt: f32, frozen: bool) {
        if frozen {
            return;
        }
        self.advance(dt);
    }

    /// The hour of day now.
    pub(super) fn hour(&self) -> f32 {
        day_night::clock_hour(&self.cfg, self.seconds as f32)
    }

    pub(super) fn environment(&self) -> crate::viewer::Environment {
        crate::viewer::Environment {
            hour: self.hour(),
            seconds_per_hour: self.cfg.seconds_per_hour,
            sun_mode: self.sun_mode,
            sun_phase: self.sun_phase(),
            ..Default::default()
        }
    }

    pub(super) fn configure(&mut self, value: crate::viewer::Environment) {
        if !value.hour.is_finite() || !value.seconds_per_hour.is_finite() || !value.sun_phase.is_finite() {
            return;
        }
        self.sun_mode = value.sun_mode;
        self.phase = f64::from(value.sun_phase.clamp(sun_phase_limit(0), 1.0 - sun_phase_limit(0)));
        self.cfg.start_hour = day_night::wrap_hour(value.hour);
        self.cfg.seconds_per_hour = value.seconds_per_hour.clamp(10.0, 3600.0);
        self.cfg.force_time = None;
        self.seconds = 0.0;
    }

    /// The sun's phase for the hour.
    pub(super) fn sun_phase(&self) -> f32 {
        if self.sun_mode != crate::viewer::SunMode::CustomClock {
            return self.phase as f32;
        }
        day_night::sun_phase(self.hour(), &self.cfg, sun_phase_limit(0))
    }

    /// Whether the lights are on at the hour.
    pub(super) fn lights_on(&self) -> bool {
        day_night::lights_on(self.hour(), &self.cfg)
    }

    /// Records the lights state and reports whether it changed since the last switch.
    pub(super) fn lights_changed(&mut self, on: bool) -> bool {
        let changed = self.lights != Some(on);
        self.lights = Some(on);
        changed
    }
}

impl WorldScene {
    pub(super) fn configure_day(&mut self, value: crate::viewer::Environment) {
        self.day.configure(value);
        self.update_day(0.0, true);
        self.reflection_last = None;
    }

    /// Moves the day clock on by `sim_dt`, sets the sun for the hour, and switches the headlights when the clock
    /// crosses the lights-on or lights-off hour. A drive that is not yet placed keeps its own state.
    pub(super) fn update_day(&mut self, sim_dt: f32, frozen: bool) {
        let held = frozen || self.day.held();
        self.day.update(sim_dt, held);
        self.sun.set_phase(self.day.sun_phase());
        let on = self.day.lights_on();
        let Some(drive) = self.drive.as_mut() else {
            return;
        };
        if !self.day.lights_changed(on) {
            return;
        }
        drive.set_headlights(on);
    }
}

#[cfg(test)]
mod tests {
    use super::super::drive::SpeedbreakerClock;
    use super::*;

    /// The default config: 150 real seconds per game hour, starting at noon.
    fn clock() -> DayClock {
        DayClock::new(DayNightConfig::default())
    }

    #[test]
    fn freezing_holds_lighting_and_resume_does_not_catch_up() {
        let mut day = clock();
        let before = (day.hour(), day.sun_phase(), day.lights_on());
        day.update(600.0, true);
        assert_eq!((day.hour(), day.sun_phase(), day.lights_on()), before);
        day.update(1.0, false);
        let mut expected = clock();
        expected.advance(1.0);
        assert_eq!(day.hour(), expected.hour());
        assert_eq!(day.sun_phase(), expected.sun_phase());
    }

    #[test]
    fn mod_and_lab_freeze_flags_hold_the_original_cycle_without_resetting_its_phase() {
        let mut day = clock();
        day.configure(crate::viewer::Environment {
            sun_mode: crate::viewer::SunMode::OriginalCycle,
            sun_phase: 0.3,
            ..Default::default()
        });
        day.set_held(true);
        day.update(600.0, day.held());
        assert_eq!(day.sun_phase(), 0.3);
        day.set_held(false);
        day.update(120.0, day.held());
        assert!((day.sun_phase() - 0.4).abs() < 1e-6);
    }

    /// One real frame at 60 Hz: the speedbreaker-dilated delta times GameSpeed, the same product
    /// `update_simulation` returns as `sim_dt`.
    fn sim_frame(breaker: &mut SpeedbreakerClock, active: bool, game_speed: f32) -> f32 {
        breaker.advance(active, 1.0 / 60.0) * game_speed
    }

    /// Pause, loading and the garage all hand the day clock a zero `sim_dt` (see `update_simulation` and the
    /// `Pausable` early return, whose own test lives in `frontend/scene.rs`). A zero delta must not move the day.
    #[test]
    fn a_zero_simulation_delta_does_not_advance_the_day() {
        let mut day = clock();
        let before = day.hour();
        for _ in 0..3600 {
            day.advance(0.0);
        }
        assert_eq!(day.hour(), before);
        assert_eq!(day.seconds, 0.0);
    }

    /// Unscaled, one game hour is 150 real seconds: 150 simulated seconds take noon to 13:00.
    #[test]
    fn normal_speed_runs_the_day_unscaled() {
        let mut day = clock();
        let mut breaker = SpeedbreakerClock::default();
        for _ in 0..(150 * 60) {
            day.advance(sim_frame(&mut breaker, false, 1.0));
        }
        assert!((day.seconds - 150.0).abs() < 1e-2, "{}", day.seconds);
        assert!((day.hour() - 13.0).abs() < 1e-4, "{}", day.hour());
    }

    /// Active speedbreaker: after the entry ramp the clock runs at the quarter-speed dilation, so 60 real
    /// seconds give 15.140625 simulated seconds (0.234375 across the ramp, then 0.25 per second).
    #[test]
    fn an_active_speedbreaker_dilates_the_day_clock() {
        let mut day = clock();
        let mut breaker = SpeedbreakerClock::default();
        for _ in 0..3600 {
            day.advance(sim_frame(&mut breaker, true, 1.0));
        }
        assert!((day.seconds - 15.140_625).abs() < 1e-2, "{}", day.seconds);
    }

    /// GameSpeed multiplies the delta before the day clock, so the day moves in proportion to it.
    #[test]
    fn game_speed_scales_the_day_clock_linearly() {
        let mut slow = clock();
        let mut fast = clock();
        let mut slow_breaker = SpeedbreakerClock::default();
        let mut fast_breaker = SpeedbreakerClock::default();
        for _ in 0..600 {
            slow.advance(sim_frame(&mut slow_breaker, false, 0.5));
            fast.advance(sim_frame(&mut fast_breaker, false, 2.0));
        }
        assert!((fast.seconds - 4.0 * slow.seconds).abs() < 1e-2, "{} vs {}", fast.seconds, slow.seconds);
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;

    #[test]
    fn manual_time_and_rate_override_a_forced_clock_without_catching_up() {
        let mut day = DayClock::new(DayNightConfig { force_time: Some(3.0), ..Default::default() });
        day.advance(90.0);
        assert_eq!(day.hour(), 3.0);
        day.configure(crate::viewer::Environment { hour: 22.0, seconds_per_hour: 60.0, ..Default::default() });
        assert_eq!(day.hour(), 22.0);
        assert!(day.lights_on());
        day.update(600.0, true);
        assert_eq!(day.hour(), 22.0);
        day.update(120.0, false);
        assert_eq!(day.hour(), 0.0);
        let mut value = day.environment();
        value.seconds_per_hour = 30.0;
        day.configure(value);
        assert_eq!(day.hour(), 0.0);
        day.advance(30.0);
        assert_eq!(day.hour(), 1.0);
        assert!(!day_night::lights_on(12.0, &DayNightConfig::default()));
    }

    #[test]
    fn clock_controls_are_finite_and_frame_rate_independent() {
        for fps in [5, 30, 60, 144, 360] {
            let mut day = DayClock::new(DayNightConfig::default());
            day.configure(crate::viewer::Environment { hour: 18.0, seconds_per_hour: 60.0, ..Default::default() });
            for _ in 0..fps * 120 {
                day.advance(1.0 / fps as f32);
            }
            assert!((day.hour() - 20.0).abs() < 1e-5, "fps {fps}: {}", day.hour());
            let before = day.environment();
            day.configure(crate::viewer::Environment { hour: f32::NAN, seconds_per_hour: 0.0, ..Default::default() });
            day.advance(f32::INFINITY);
            assert_eq!(day.environment(), before);
        }
    }
}

#[cfg(test)]
mod comparison_tests {
    use super::*;
    use crate::viewer::{Environment, SunMode};

    #[test]
    fn original_cycle_reflects_at_limits_and_matches_high_frame_rates() {
        let mut expected: Option<f32> = None;
        for fps in [30, 60, 144, 360] {
            let mut day = DayClock::new(DayNightConfig::default());
            day.configure(Environment { sun_mode: SunMode::OriginalCycle, sun_phase: 0.94, ..Default::default() });
            for _ in 0..fps * 30 {
                day.update(1.0 / fps as f32, false);
            }
            assert!(day.direction < 0.0, "cycle must reverse at the high endpoint");
            assert!((day.sun_phase() - 0.935).abs() < 1e-6);
            if let Some(phase) = expected {
                assert!((day.sun_phase() - phase).abs() < 1e-6);
            }
            expected = Some(day.sun_phase());
            let before = day.sun_phase();
            day.update(1000.0, true);
            assert_eq!(day.sun_phase(), before);
            day.update(1.0, false);
            assert!((day.sun_phase() - (before - 1.0 / 1200.0)).abs() < 1e-6);
        }
    }

    #[test]
    fn fixed_phase_holds_without_changing_custom_clock_mapping() {
        let mut day = DayClock::new(DayNightConfig::default());
        day.configure(Environment { sun_mode: SunMode::FixedPhase, sun_phase: 0.3, ..Default::default() });
        day.advance(3600.0);
        assert_eq!(day.sun_phase(), 0.3);
        let mut value = day.environment();
        value.sun_mode = SunMode::CustomClock;
        value.hour = 12.0;
        day.configure(value);
        assert_eq!(day.sun_phase(), 0.5);
        let before = day.environment();
        value.sun_phase = f32::NAN;
        day.configure(value);
        assert_eq!(day.environment(), before);
    }
}
