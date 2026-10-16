//! `nfsmw.pursuit` 0.1: the heat level, the cop waves per heat, and the chase (catalogue: docs/modding/api-nfsmw.md,
//! "Pursuit and heat").
//!
//! The table is [`BbNfsmwPursuit`]. Every function takes `ctx` first (the host's pointer for this mod) and returns a
//! `BbStatus`. A cop wave is a [`BbHeatRow`] header plus an array of [`BbCopRecord`]; the array is passed beside the
//! header, so the header itself holds no pointer.

use core::ffi::c_void;

use blackbox_modding::abi::{BbIface, BbStatus};

use super::shared::BbName;

/// One kind of cop car in a wave: the car (its `pvehicle` collection name), how many, and its spawn weight in percent.
/// A plain array element, so it has no size word. Its layout is fixed for the whole 0.x line.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbCopRecord {
    /// How many of this car the wave has.
    pub count: u32,
    /// Spawn weight, percent (0 to 100).
    pub chance: u32,
    /// The car's `pvehicle` collection name, for example `copmidsize`.
    pub name: BbName,
}

impl BbCopRecord {
    /// A record for the car named `car`. `None` when the name is longer than the host accepts.
    pub fn new(car: &str, count: u32, chance: u32) -> Option<Self> {
        Some(BbCopRecord { count, chance, name: BbName::new(car)? })
    }
}

/// The header of one cop wave (one row of `heattable` or `racetable`). Size-prefixed: `size` is the number of bytes
/// the writer filled in. The records of the wave are passed beside it, `cop_count` of them.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbHeatRow {
    /// Bytes the writer filled in.
    pub size: u32,
    /// Number of cop records in the wave.
    pub cop_count: u32,
    /// Patrol cars on the street before the chase.
    pub patrol_cars: u32,
    /// Cops that have to be evaded to end the wave.
    pub full_engagement_cops: u32,
    /// Seconds between the first four spawns.
    pub first_four_interval: f32,
    /// Seconds between the later spawns.
    pub spawn_interval: f32,
}

/// `heat_get`: write the heat (1 to 10 with its fraction) to `out`.
pub type BbPursuitHeatGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut f32) -> BbStatus;
/// `heat_set`: set the heat. The game clamps it to the pursuit's base and maximum.
pub type BbPursuitHeatSetFn = unsafe extern "C" fn(ctx: *mut c_void, heat: f32) -> BbStatus;
/// `cop_table_get`: write the header of the wave of `table` at `level` to `out`, and as many records as `cops_cap` allows
/// to `cops`. `out.cop_count` is the total, so a caller passes `cops_cap = 0` to learn it first.
pub type BbPursuitCopTableGetFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    table: u32,
    level: u32,
    out: *mut BbHeatRow,
    cops: *mut BbCopRecord,
    cops_cap: u32,
) -> BbStatus;
/// `cop_table_set`: replace the wave of `table` at `level`. `cops_len` records are read from `cops`, and must equal
/// `row.cop_count`. Applies to the next wave, not the one running.
pub type BbPursuitCopTableSetFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    table: u32,
    level: u32,
    row: *const BbHeatRow,
    cops: *const BbCopRecord,
    cops_len: u32,
) -> BbStatus;
/// `start`: start a pursuit at `level`. `near` is a vehicle handle the cops come from, or 0 for none.
pub type BbPursuitStartFn = unsafe extern "C" fn(ctx: *mut c_void, level: u32, near: u64) -> BbStatus;
/// `end`: end the running pursuit by `mode` (an `EndMode` number).
pub type BbPursuitEndFn = unsafe extern "C" fn(ctx: *mut c_void, mode: u32) -> BbStatus;
/// `is_pursuing`: write 1 to `out` while a chase runs, 0 otherwise.
pub type BbPursuitIsPursuingFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u8) -> BbStatus;
/// `busted_get`: write the busted meter, 0 to 1, to `out`.
pub type BbPursuitBustedGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut f32) -> BbStatus;
/// `cops_in_pursuit`: write the number of cops in the running pursuit to `out`.
pub type BbPursuitCopsInPursuitFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;

/// The `nfsmw.pursuit` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbNfsmwPursuit {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the heat.
    pub heat_get: Option<BbPursuitHeatGetFn>,
    /// Set the heat.
    pub heat_set: Option<BbPursuitHeatSetFn>,
    /// Read a cop wave.
    pub cop_table_get: Option<BbPursuitCopTableGetFn>,
    /// Replace a cop wave.
    pub cop_table_set: Option<BbPursuitCopTableSetFn>,
    /// Start a pursuit.
    pub start: Option<BbPursuitStartFn>,
    /// End the pursuit.
    pub end: Option<BbPursuitEndFn>,
    /// Ask whether a chase runs.
    pub is_pursuing: Option<BbPursuitIsPursuingFn>,
    /// Read the busted meter.
    pub busted_get: Option<BbPursuitBustedGetFn>,
    /// Count the cops of the running pursuit.
    pub cops_in_pursuit: Option<BbPursuitCopsInPursuitFn>,
}
