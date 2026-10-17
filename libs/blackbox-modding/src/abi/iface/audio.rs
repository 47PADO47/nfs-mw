//! `bb.audio` 1.0: play a one-shot sound by bank and event name (catalogue: docs/modding/api-world.md#audio).
//!
//! The game plays sounds by bank and index in its sound viewer. Event names are not looked up yet, so the game answers
//! `BB_ERR_UNSUPPORTED`. The table is offered now, so a mod's code does not change when the lookup lands.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// `play`: play the event `event` of bank `bank` once at `volume` (0 to 1). The handle goes to `out_id`.
pub type BbAudioPlayFn =
    unsafe extern "C" fn(ctx: *mut c_void, bank: BbStr, event: BbStr, volume: f32, out_id: *mut u64) -> BbStatus;

/// The `bb.audio` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbAudio {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Play a sound.
    pub play: Option<BbAudioPlayFn>,
}
