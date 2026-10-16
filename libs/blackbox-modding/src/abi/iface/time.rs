//! `bb.time` 1.0: read and set the world's time of day (catalogue: docs/modding/api.md#time-of-day).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::BbStatus;

/// The clock as the host reports it. Plain data, written into the caller's memory by `get`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbTimeOfDay {
    /// Hour of the day, 0 to 24.
    pub hour: f32,
    /// Real seconds one in-world hour takes. Zero means the clock is stopped.
    pub seconds_per_hour: f32,
    /// Non-zero while the hour is held.
    pub frozen: u8,
    /// Non-zero while the headlights are on by the day-night rule.
    pub lights_on: u8,
    /// Padding, zero.
    pub _pad: [u8; 2],
}

/// `get`: write the current clock to `out`.
pub type BbTimeGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbTimeOfDay) -> BbStatus;
/// `set_hour`: set the hour, wrapped into 0 to 24.
pub type BbTimeSetHourFn = unsafe extern "C" fn(ctx: *mut c_void, hour: f32) -> BbStatus;
/// `set_day_length`: set the real seconds per in-world hour. Zero stops the clock.
pub type BbTimeSetDayLengthFn = unsafe extern "C" fn(ctx: *mut c_void, seconds_per_hour: f32) -> BbStatus;
/// `freeze`: hold the hour (non-zero) or let it run (zero).
pub type BbTimeFreezeFn = unsafe extern "C" fn(ctx: *mut c_void, on: u8) -> BbStatus;

/// The `bb.time` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbTime {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the clock.
    pub get: Option<BbTimeGetFn>,
    /// Set the hour.
    pub set_hour: Option<BbTimeSetHourFn>,
    /// Set the speed of the clock.
    pub set_day_length: Option<BbTimeSetDayLengthFn>,
    /// Freeze or unfreeze the hour.
    pub freeze: Option<BbTimeFreezeFn>,
}
