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
