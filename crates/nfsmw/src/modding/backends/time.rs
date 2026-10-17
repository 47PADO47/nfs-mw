//! `bb.time`: the day clock a mod reads and sets.
//!
//! The clock is the scene's (`Scene::environment`, `Scene::set_environment`, `Scene::set_time_held`). Each frame the mods
//! read a copy of it, taken by [`read`] before they run. A change they ask for is queued as a [`ClockRequest`] and made on
//! the scene by [`apply`] after they have run, so the scene sees it on its next update. The copy is updated at once too, so
//! a mod reads back what it just set.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_NOT_READY, BbTimeOfDay};
use blackbox_modding::host::{BackendResult, TimeBackend};
use nfsmw_data::day_night::{self, DayNightConfig};

use super::bridge::Bridge;
use crate::viewer::{Environment, Scene};

/// The real seconds one in-world hour may take. The day clock clamps a rate to this range (`scenes/world/day.rs`).
const RATE_RANGE: (f32, f32) = (10.0, 3600.0);

/// The day clock as the mods see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    /// Hour of the day, 0 to 24.
    pub hour: f32,
    /// Real seconds one in-world hour takes, kept while the clock is held.
    pub seconds_per_hour: f32,
    /// A mod has held the hour.
    pub held: bool,
    /// The headlights are on by the day-night rule at this hour.
    pub lights_on: bool,
}

/// A change to the clock, made on the scene by [`apply`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockRequest {
    SetHour(f32),
    SetRate(f32),
    Hold(bool),
}

/// The `bb.time` port of the game: one per mod table, sharing the bridge.
pub struct TimePort(pub(super) Rc<Bridge>);

/// The clock of `scene`, or `None` when the scene has none (the menus).
pub fn read(scene: &dyn Scene) -> Option<Clock> {
    let env = scene.environment()?;
    Some(Clock {
        hour: env.hour,
        seconds_per_hour: env.seconds_per_hour,
        held: scene.time_held(),
        lights_on: lights_on(env.hour),
    })
}

/// Make the queued changes on `scene`, in order. The hour and the rate go to the scene in one call; a hold goes on its
/// own. A scene without a clock takes none of them.
pub fn apply(requests: Vec<ClockRequest>, scene: &mut dyn Scene) {
    let Some(env) = scene.environment() else { return };
    let (mut hour, mut rate, mut moved) = (env.hour, env.seconds_per_hour, false);
    for request in requests {
        match request {
            ClockRequest::SetHour(value) => {
                hour = value;
                moved = true;
            }
            ClockRequest::SetRate(value) => {
                rate = value;
                moved = true;
            }
            ClockRequest::Hold(on) => scene.set_time_held(on),
        }
    }
    if !moved {
        return;
    }
    scene.set_environment(Environment { hour, seconds_per_hour: rate, ..env });
}

/// Whether the headlights are on at `hour`, by the game's default day-night curves (the world's clock uses them).
fn lights_on(hour: f32) -> bool {
    day_night::lights_on(hour, &DayNightConfig::default())
}

/// What a mod reads: a held clock shows a rate of 0, as the ABI says a stopped clock does.
fn time_of_day(clock: Clock) -> BbTimeOfDay {
    BbTimeOfDay {
        hour: clock.hour,
        seconds_per_hour: if clock.held { 0.0 } else { clock.seconds_per_hour },
        frozen: u8::from(clock.held),
        lights_on: u8::from(clock.lights_on),
        _pad: [0; 2],
    }
}

impl TimePort {
    /// Edit the copy of the clock. It must exist: a scene with no clock refuses the call.
    fn change(&self, edit: impl FnOnce(&mut Clock)) -> BackendResult<()> {
        let mut clock = self.0.clock.get().ok_or(BB_ERR_NOT_READY)?;
        edit(&mut clock);
        self.0.clock.set(Some(clock));
        Ok(())
    }
}

impl TimeBackend for TimePort {
    fn get(&self, _owner: &str) -> BackendResult<BbTimeOfDay> {
        let clock = self.0.clock.get().ok_or(BB_ERR_NOT_READY)?;
        Ok(time_of_day(clock))
    }

    fn set_hour(&self, _owner: &str, hour: f32) -> BackendResult<()> {
        let hour = hour.rem_euclid(24.0);
        self.change(|clock| {
            clock.hour = hour;
            clock.lights_on = lights_on(hour);
        })?;
        self.0.clock_requests.borrow_mut().push(ClockRequest::SetHour(hour));
        Ok(())
    }

    fn set_day_length(&self, owner: &str, seconds_per_hour: f32) -> BackendResult<()> {
        // Zero stops the clock, as the API says (`docs/modding/api.md`, time of day).
        if seconds_per_hour == 0.0 {
            return self.freeze(owner, true);
        }
        let rate = seconds_per_hour.clamp(RATE_RANGE.0, RATE_RANGE.1);
        self.change(|clock| clock.seconds_per_hour = rate)?;
        self.0.clock_requests.borrow_mut().push(ClockRequest::SetRate(rate));
        Ok(())
    }

    fn freeze(&self, _owner: &str, on: bool) -> BackendResult<()> {
        self.change(|clock| clock.held = on)?;
        self.0.clock_requests.borrow_mut().push(ClockRequest::Hold(on));
        Ok(())
    }
}
