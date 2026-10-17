//! `bb.world` 1.0: the track and the district the player is in, the rain, and the traffic density (catalogue:
//! docs/modding/api-world.md#world-state).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus};

/// `track` and `section`: the name is written to `sink` during the call (the sink is valid only for it). The host
/// answers `BB_ERR_NOT_READY` while no world is loaded.
pub type BbWorldTextFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;
/// `set_rain`: `always` non-zero keeps it raining; zero leaves it to the weather. `amount` is 0 to 1.
pub type BbWorldSetRainFn = unsafe extern "C" fn(ctx: *mut c_void, always: u8, amount: f32) -> BbStatus;
/// `set_traffic`: keep `cars` computer-driven cars on the road, `cop_share` percent of them patrol cops (0 to 100).
pub type BbWorldSetTrafficFn = unsafe extern "C" fn(ctx: *mut c_void, cars: u32, cop_share: u32) -> BbStatus;

/// The `bb.world` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbWorld {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// The name of the track.
    pub track: Option<BbWorldTextFn>,
    /// The name of the district at the player's position.
    pub section: Option<BbWorldTextFn>,
    /// Set the rain.
    pub set_rain: Option<BbWorldSetRainFn>,
    /// Set the traffic density.
    pub set_traffic: Option<BbWorldSetTrafficFn>,
}
