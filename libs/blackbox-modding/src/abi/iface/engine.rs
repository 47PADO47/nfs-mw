//! `bb.engine` 1.0: the player's engine switch (catalogue: docs/modding/api-engine.md).
//!
//! A mod reads the driver's pedals and the engine's phase, and switches the player's engine off or on. An engine that is
//! off takes no throttle and no nitrous, shows no revs and makes no engine sound, and the car coasts; the switch changes
//! nothing else. Starting plays a sound the mod names, a WAV file of its own `files/` folder.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus, BbStr};

/// `BbEngineState::phase`: the engine is off.
pub const BB_ENGINE_OFF: u32 = 0;
/// The engine is starting: it is not running yet, and runs when the start delay is over.
pub const BB_ENGINE_STARTING: u32 = 1;
/// The engine is running.
pub const BB_ENGINE_RUNNING: u32 = 2;

/// The player's engine and pedals as the host reads them. Size-prefixed: the host writes the first `size` bytes of its
/// layout, so a mod built against an older minor gets the fields it knows.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbEngineState {
    /// Bytes the caller allows the host to write.
    pub size: u32,
    /// `BB_ENGINE_OFF`, `BB_ENGINE_STARTING` or `BB_ENGINE_RUNNING`.
    pub phase: u32,
    /// Non-zero while the player drives free of any event: no race, no pursuit, no cutscene, no menu over the car.
    pub free_roam: u32,
    /// The driver's throttle pedal, 0 to 1, whether or not the engine takes it.
    pub throttle: f32,
    /// The driver's brake pedal, 0 to 1.
    pub brake: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
}

/// `state`: write the player's engine and pedals to `out`. `BB_ERR_NOT_READY` while no car is driven.
pub type BbEngineStateFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbEngineState) -> BbStatus;
/// `car`: the id of the player's car (`POR911CARR12`) is written to `sink` during the call. `BB_ERR_NOT_READY` while no
/// car is driven.
pub type BbEngineCarFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;
/// `stop`: switch the engine off. Already off or starting: it is off.
pub type BbEngineStopFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;
/// `start`: start the engine. It plays `sound` (a WAV path in the mods' `files/` layers; empty for none) at `gain` (0 to
/// 8) now, and runs after `delay` seconds (0 to 10; 0 is at once). Already running: nothing happens.
pub type BbEngineStartFn = unsafe extern "C" fn(ctx: *mut c_void, sound: BbStr, gain: f32, delay: f32) -> BbStatus;

/// The `bb.engine` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbEngine {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the engine and the pedals.
    pub state: Option<BbEngineStateFn>,
    /// The id of the player's car.
    pub car: Option<BbEngineCarFn>,
    /// Switch the engine off.
    pub stop: Option<BbEngineStopFn>,
    /// Start the engine.
    pub start: Option<BbEngineStartFn>,
}
