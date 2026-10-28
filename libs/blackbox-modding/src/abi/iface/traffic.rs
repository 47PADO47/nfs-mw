//! `bb.traffic` 1.0: the traffic cars that mods added, listed and placed on the road (catalogue: docs/modding/api-traffic.md).
//!
//! A mod adds a traffic car with a car manifest (`role = "traffic"`, docs/modding/api-content.md); the game's traffic manager
//! then puts it on the roads among its own cars, by the density, population and cop-share settings. This interface is for a
//! mod that wants more: it lists the cars of the traffic pool with the numbers each appears with, and asks for one by name at a
//! point of the map (a scripted street scene, a test).

use core::ffi::c_void;

use super::BbIface;
use super::vehicles::BbVec3;
use crate::abi::{BbSinkFn, BbStatus, BbStr};

/// Separates the fields of a line of the list `list` writes.
pub const BB_TRAFFIC_FIELD: char = '\t';

/// `list`: write the cars of the traffic pool to `sink` during the call, one line per car, in the order the mods registered
/// them. A line is `id`, `class`, `weight`, `max_instances`, `speed_factor` and `patterns` (names joined by `,`, empty for
/// every pattern), separated by [`BB_TRAFFIC_FIELD`], with the lines separated by `\n`. No car is an empty text.
pub type BbTrafficListFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;
/// `spawn`: put the traffic car `car` (an id the list names, any case) on the lane nearest `position` (the height is not
/// used), heading `heading` radians from +x. Queued: the scene places it after the mods have run, once the car's model is
/// loaded (a few seconds the first time), and drops the request after 30 seconds. `BB_ERR_NOT_FOUND` for a car that is not in
/// the pool, `BB_ERR_ARG` for a point or heading that is not finite, `BB_ERR_NOT_READY` while no world is loaded.
pub type BbTrafficSpawnFn =
    unsafe extern "C" fn(ctx: *mut c_void, car: BbStr, position: BbVec3, heading: f32) -> BbStatus;

/// The `bb.traffic` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbTraffic {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// List the cars of the traffic pool.
    pub list: Option<BbTrafficListFn>,
    /// Ask for a traffic car at a point.
    pub spawn: Option<BbTrafficSpawnFn>,
}
