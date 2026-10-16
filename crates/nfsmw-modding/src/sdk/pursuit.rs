//! `Pursuit`: the heat, the cop waves per heat, and the chase (catalogue: docs/modding/api-nfsmw.md, "Pursuit and heat").
//!
//! Obtained from `ctx.nfsmw_pursuit()` (see [`NfsmwExt`](super::NfsmwExt)). A cop wave is a [`HeatRow`] of owned values;
//! the SDK converts it to and from the ABI record.

use core::mem::size_of;

use blackbox_modding::sdk::{Error, ModResult, VehicleId};

use super::ffi::pursuit::{self as host, PursuitFns};
use crate::abi::{BbCopRecord, BbHeatRow};

pub use crate::abi::{CopTable, EndMode};

/// One kind of cop car in a wave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopRecord {
    /// The car's `pvehicle` collection name, for example `copmidsize`.
    pub car: String,
    /// How many of it the wave has.
    pub count: u32,
    /// Spawn weight, percent (0 to 100).
    pub chance: u32,
}

/// One cop wave: the cop records and the wave's numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatRow {
    /// The cop records, in order.
    pub cops: Vec<CopRecord>,
    /// Patrol cars on the street before the chase.
    pub patrol_cars: u32,
    /// Cops that have to be evaded to end the wave.
    pub full_engagement_cops: u32,
    /// Seconds between the first four spawns.
    pub first_four_interval: f32,
    /// Seconds between the later spawns.
    pub spawn_interval: f32,
}

impl HeatRow {
    /// The owned wave of an ABI header and its records.
    fn from_abi(row: &BbHeatRow, cops: &[BbCopRecord]) -> ModResult<Self> {
        let cops = cops
            .iter()
            .map(|cop| {
                let car = cop.name.text().ok_or(Error::Host)?.to_owned();
                Ok(CopRecord { car, count: cop.count, chance: cop.chance })
            })
            .collect::<ModResult<Vec<_>>>()?;
        Ok(HeatRow {
            cops,
            patrol_cars: row.patrol_cars,
            full_engagement_cops: row.full_engagement_cops,
            first_four_interval: row.first_four_interval,
            spawn_interval: row.spawn_interval,
        })
    }

    /// The ABI header and records of this wave. `Error::Arg` for a car name longer than the host accepts.
    fn to_abi(&self) -> ModResult<(BbHeatRow, Vec<BbCopRecord>)> {
        let cops = self
            .cops
            .iter()
            .map(|cop| BbCopRecord::new(&cop.car, cop.count, cop.chance).ok_or(Error::Arg))
            .collect::<ModResult<Vec<_>>>()?;
        let row = BbHeatRow {
            size: size_of::<BbHeatRow>() as u32,
            cop_count: cops.len() as u32,
            patrol_cars: self.patrol_cars,
            full_engagement_cops: self.full_engagement_cops,
            first_four_interval: self.first_four_interval,
            spawn_interval: self.spawn_interval,
        };
        Ok((row, cops))
    }
}

/// The `nfsmw.pursuit` interface of the host, as a mod uses it.
#[derive(Clone, Copy)]
pub struct Pursuit {
    fns: PursuitFns,
}

impl Pursuit {
    pub(crate) fn new(fns: PursuitFns) -> Self {
        Pursuit { fns }
    }

    /// The heat, 1 to 10 with its fraction.
    pub fn heat(&self) -> ModResult<f32> {
        host::heat_get(&self.fns)
    }

    /// Set the heat. The host clamps it to the pursuit's base and maximum; a non-finite value is `Error::Arg`.
    pub fn set_heat(&self, heat: f32) -> ModResult<()> {
        host::heat_set(&self.fns, heat)
    }

    /// The cop wave of `table` at `level` (1 to 10).
    pub fn cop_table(&self, table: CopTable, level: u32) -> ModResult<HeatRow> {
        let (row, cops) = host::cop_table_get(&self.fns, table.raw(), level)?;
        HeatRow::from_abi(&row, &cops)
    }

    /// Replace the cop wave of `table` at `level`. It applies to the next wave, not the one running.
    pub fn set_cop_table(&self, table: CopTable, level: u32, row: &HeatRow) -> ModResult<()> {
        let (header, cops) = row.to_abi()?;
        host::cop_table_set(&self.fns, table.raw(), level, &header, &cops)
    }

    /// Start a pursuit at `level`, with the cops coming from `near` when it is given.
    pub fn start(&self, level: u32, near: Option<VehicleId>) -> ModResult<()> {
        host::start(&self.fns, level, near.map_or(0, VehicleId::raw))
    }

    /// End the running pursuit.
    pub fn end(&self, mode: EndMode) -> ModResult<()> {
        host::end(&self.fns, mode.raw())
    }

    /// Whether a chase runs.
    pub fn is_pursuing(&self) -> ModResult<bool> {
        host::is_pursuing(&self.fns)
    }

    /// The busted meter, 0 to 1.
    pub fn busted(&self) -> ModResult<f32> {
        host::busted_get(&self.fns)
    }

    /// The number of cops in the running pursuit.
    pub fn cops(&self) -> ModResult<u32> {
        host::cops_in_pursuit(&self.fns)
    }
}
