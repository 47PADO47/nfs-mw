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
    seconds: f32,
    /// The lights state last applied to the drive, so a manual headlight toggle holds until the next switch.
    lights: Option<bool>,
}

impl DayClock {
    pub(super) fn new(cfg: DayNightConfig) -> Self {
        Self { cfg, seconds: 0.0, lights: None }
    }

    /// Moves the clock on by `dt` simulated seconds.
    pub(super) fn advance(&mut self, dt: f32) {
        self.seconds += dt;
    }

    /// The hour of day now.
    pub(super) fn hour(&self) -> f32 {
        day_night::clock_hour(&self.cfg, self.seconds)
    }

    /// The sun's phase for the hour.
    pub(super) fn sun_phase(&self) -> f32 {
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
    /// Moves the day clock on by `sim_dt`, sets the sun for the hour, and switches the headlights when the clock
    /// crosses the lights-on or lights-off hour. A drive that is not yet placed keeps its own state.
    pub(super) fn update_day(&mut self, sim_dt: f32) {
        self.day.advance(sim_dt);
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
