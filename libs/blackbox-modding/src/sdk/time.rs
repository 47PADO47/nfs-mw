//! `Time`: read and set the world's time of day (catalogue: docs/modding/api.md#time-of-day).

use super::ModResult;
use super::ffi::time::{self as host_time, TimeFns};
use crate::abi::BbTimeOfDay;

/// The world clock, as a mod reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeOfDay {
    /// Hour of the day, 0 to 24.
    pub hour: f32,
    /// Real seconds one in-world hour takes. Zero means the clock is stopped.
    pub seconds_per_hour: f32,
    /// Whether the hour is held.
    pub frozen: bool,
    /// Whether the headlights are on by the day-night rule.
    pub lights_on: bool,
}

impl TimeOfDay {
    fn from_abi(raw: BbTimeOfDay) -> Self {
        TimeOfDay {
            hour: raw.hour,
            seconds_per_hour: raw.seconds_per_hour,
            frozen: raw.frozen != 0,
            lights_on: raw.lights_on != 0,
        }
    }
}

/// The `bb.time` interface of the host, as a mod uses it. Obtained from `Ctx::time`.
#[derive(Clone, Copy)]
pub struct Time {
    fns: TimeFns,
}

impl Time {
    pub(crate) fn new(fns: TimeFns) -> Self {
        Time { fns }
    }

    /// The current clock.
    pub fn get(&self) -> ModResult<TimeOfDay> {
        host_time::get(&self.fns).map(TimeOfDay::from_abi)
    }

    /// Set the hour. The host wraps it into 0 to 24.
    pub fn set_hour(&self, hour: f32) -> ModResult<()> {
        host_time::set_hour(&self.fns, hour)
    }

    /// Set the real seconds per in-world hour. Zero stops the clock.
    pub fn set_day_length(&self, seconds_per_hour: f32) -> ModResult<()> {
        host_time::set_day_length(&self.fns, seconds_per_hour)
    }

    /// Hold the hour (`true`) or let it run (`false`).
    pub fn freeze(&self, on: bool) -> ModResult<()> {
        host_time::freeze(&self.fns, on)
    }
}
