//! The cop waves the mods set for the next chase (`nfsmw.pursuit` `cop_table_set`). A wave a mod sets is kept for the session,
//! and a chase that starts takes it in place of the install's row for that heat (`ai/commands.rs` `begin_pursuit`, which
//! also loads the cop cars the waves name). A chase that is running keeps its rows. This module also converts a wave
//! between the install's form and the ABI's.

use std::cell::RefCell;
use std::mem::size_of;

use nfsmw_data::pursuit::{CopRecord, HeatRow};
use nfsmw_modding::abi::{BbCopRecord, BbHeatRow, CopTable};
use nfsmw_modding::host::HeatRowSpec;

/// A wave a mod set: the table's number, the heat level and the row.
struct Override {
    table: u32,
    level: u32,
    row: HeatRow,
}

thread_local! {
    static OVERRIDES: RefCell<Vec<Override>> = const { RefCell::new(Vec::new()) };
}

/// The wave of `table` at `level` a mod set, if one is set.
pub(crate) fn get(table: CopTable, level: u32) -> Option<HeatRow> {
    let table = table.raw();
    OVERRIDES.with(|overrides| {
        overrides
            .borrow()
            .iter()
            .find(|known| known.table == table && known.level == level)
            .map(|known| known.row.clone())
    })
}

/// Keep `row` as the wave of `table` at `level`, from the next chase on. It replaces a wave set before.
pub(crate) fn set(table: CopTable, level: u32, row: HeatRow) {
    let table = table.raw();
    OVERRIDES.with(|overrides| {
        let mut overrides = overrides.borrow_mut();
        overrides.retain(|known| !(known.table == table && known.level == level));
        overrides.push(Override { table, level, row });
    });
}

/// The rows of a new chase: each heat level a mod set on the heat table takes the place of the install's row for it. The
/// rows are indexed by level - 1, as `Pursuit` reads them.
pub(crate) fn with_heat_overrides(mut rows: Vec<HeatRow>) -> Vec<HeatRow> {
    let heat = CopTable::Roam.raw();
    OVERRIDES.with(|overrides| {
        for known in overrides.borrow().iter().filter(|known| known.table == heat) {
            let Some(index) = known.level.checked_sub(1) else {
                continue;
            };
            if let Some(slot) = rows.get_mut(index as usize) {
                *slot = known.row.clone();
            }
        }
    });
    rows
}

/// The rows without the cop records of the car types in `missing` (compared without case). A chase leaves out the wave
/// entries whose model did not load, so the picker never asks for a car that cannot spawn. The helicopter is not a car
/// and stays in its wave.
pub(crate) fn without_models(mut rows: Vec<HeatRow>, missing: &[String]) -> Vec<HeatRow> {
    if missing.is_empty() {
        return rows;
    }
    for row in &mut rows {
        row.cops.retain(|cop| !missing.iter().any(|name| name.eq_ignore_ascii_case(&cop.name)));
    }
    rows
}

/// A wave as the ABI gives it: the header, and the records with it. A record whose name is too long for the ABI is left out.
pub(crate) fn spec_of(row: &HeatRow) -> HeatRowSpec {
    let cops: Vec<BbCopRecord> =
        row.cops.iter().filter_map(|cop| BbCopRecord::new(&cop.name, cop.count, cop.chance)).collect();
    HeatRowSpec {
        row: BbHeatRow {
            size: size_of::<BbHeatRow>() as u32,
            cop_count: cops.len() as u32,
            patrol_cars: row.patrol_cars,
            full_engagement_cops: row.full_engagement_cops,
            first_four_interval: row.time_between_first_four_spawns,
            spawn_interval: row.time_between_cop_spawns,
        },
        cops,
    }
}

/// A wave a mod sent, in the install's form. `None` when a record's name is not text.
pub(crate) fn row_of(spec: &HeatRowSpec) -> Option<HeatRow> {
    let cops = spec
        .cops
        .iter()
        .map(|cop| Some(CopRecord { name: cop.name.text()?.to_owned(), count: cop.count, chance: cop.chance }))
        .collect::<Option<Vec<CopRecord>>>()?;
    Some(HeatRow {
        cops,
        patrol_cars: spec.row.patrol_cars,
        full_engagement_cops: spec.row.full_engagement_cops,
        time_between_first_four_spawns: spec.row.first_four_interval,
        time_between_cop_spawns: spec.row.spawn_interval,
    })
}

#[cfg(test)]
mod tests;
