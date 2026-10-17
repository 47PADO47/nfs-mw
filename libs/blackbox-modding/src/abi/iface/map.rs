//! `bb.map` 1.0: add, move and remove blips on the map (catalogue: docs/modding/api-world.md#map-blips-and-the-world-map).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// An RGBA colour, one byte per channel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbRgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha.
    pub a: u8,
}

/// A blip as a mod writes it. Size-prefixed: the host reads `style`, `active` and `position` only when the blip's size
/// covers `position`, the last field a blip has to carry.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbBlip {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Non-zero while the blip is drawn.
    pub active: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// Name of a blip style the host registered, valid for the call.
    pub style: BbStr,
    /// Position in render space (x forward, y left, z up), metres.
    pub position: [f32; 3],
}

/// `add`: add a blip. The new id is written to `out_id`.
pub type BbMapAddFn = unsafe extern "C" fn(ctx: *mut c_void, blip: *const BbBlip, out_id: *mut u64) -> BbStatus;
/// `update`: replace the state of a blip.
pub type BbMapUpdateFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64, blip: *const BbBlip) -> BbStatus;
/// `remove`: remove a blip.
pub type BbMapRemoveFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;

/// The `bb.map` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbMap {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Add a blip.
    pub add: Option<BbMapAddFn>,
    /// Update a blip.
    pub update: Option<BbMapUpdateFn>,
    /// Remove a blip.
    pub remove: Option<BbMapRemoveFn>,
}
