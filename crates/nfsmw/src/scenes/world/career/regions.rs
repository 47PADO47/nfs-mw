//! The closed regions in career mode: the barrier gates of the stage and the region unlock screen request
//! (`docs/specs/career-regions.md`). The gates come from the `Barriers` list of the stage's bin record.
//!
//! Each change of the stage switches the gate groups on the world's scenery-group set (`scenery_groups.rs`): closed
//! gates on, opened gates off. The draw cull and the collision queries read that set, so a closed gate is drawn and
//! blocks cars. A race's own walls are switched on (`race/barriers.rs`) while the gates are lifted for the run: the
//! original applies no stage gate to a race the world was made for, and the prologue's second sprint crosses one
//! (`docs/specs/career-regions.md` section 3.5).

use nfsmw_career::regions::{GateChange, GateSet, gates_in_list, group_of, region_unlock_screen};
use nfsmw_data::gameplay::Bin;

use crate::career::CareerRequest;
use crate::scenes::world::WorldScene;

/// The gates as last evaluated.
#[derive(Default)]
pub(in crate::scenes::world) struct RegionState {
    /// The stage the gates were last evaluated for; `None` before the first evaluation of the career.
    applied_stage: Option<u8>,
    /// The gates closed at `applied_stage`.
    closed: GateSet,
    /// A race runs with the stage's gates lifted: they are off in the world whatever the stage says, until the race
    /// ends (`lift`, `restore`).
    lifted: bool,
}

/// What one evaluation changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RegionUpdate {
    /// The gates that opened and closed with the stage, applied by [`WorldScene::apply_gate_change`].
    pub change: GateChange,
    /// The stage whose region unlock screen is due: the stage was reached from above and shows the screen.
    pub unlock: Option<u8>,
}

impl RegionState {
    /// The gates closed at the applied stage.
    #[allow(dead_code)] // no caller: the gates are applied as changes (`apply_gate_change`)
    pub(in crate::scenes::world) fn closed_gates(&self) -> GateSet {
        self.closed
    }

    /// Whether the gates are lifted for a race.
    pub(super) fn is_lifted(&self) -> bool {
        self.lifted
    }

    /// A race begins: the gates that are closed now must go off. `GateSet::EMPTY` when they are lifted already.
    pub(super) fn lift(&mut self) -> GateSet {
        if self.lifted {
            return GateSet::EMPTY;
        }
        self.lifted = true;
        self.closed
    }

    /// The race ends: the gates the stage closes must come back (the stage may have changed during the race, and
    /// `closed` follows it). `GateSet::EMPTY` when nothing was lifted.
    pub(super) fn restore(&mut self) -> GateSet {
        if !self.lifted {
            return GateSet::EMPTY;
        }
        self.lifted = false;
        self.closed
    }

    /// Stores the gates of `stage` when the stage differs from the applied one. `gates` is the set the stage's bin
    /// record closes. Returns `None` when the stage did not change.
    pub(super) fn evaluate(&mut self, stage: u8, gates: GateSet) -> Option<RegionUpdate> {
        if self.applied_stage == Some(stage) {
            return None;
        }
        let previous = self.applied_stage;
        let change = GateChange { opened: self.closed.minus(gates), closed: gates.minus(self.closed) };
        let unlock = unlock_due(previous, stage);
        self.applied_stage = Some(stage);
        self.closed = gates;
        Some(RegionUpdate { change, unlock })
    }
}

/// The unlock screen is due when the stage shows one and was reached from a higher stage. The first evaluation of a
/// career (a save loaded at stage 12) has no previous stage and shows nothing.
fn unlock_due(previous: Option<u8>, stage: u8) -> Option<u8> {
    let previous = previous?;
    if previous <= stage {
        return None;
    }
    region_unlock_screen(stage).map(|_| stage)
}

/// The `Barriers` list of the bin record whose `BinIndex` is `stage`. `None` when there is no record.
fn bin_barriers(bins: &[Bin], stage: u8) -> Option<&[String]> {
    bins.iter().find(|bin| bin.number == i32::from(stage)).map(|bin| bin.barriers.as_slice())
}

/// The gates a stage closes, from its bin record's `Barriers` list. No record closes none.
fn stage_gates(barriers: Option<&[String]>) -> GateSet {
    let Some(list) = barriers else { return GateSet::EMPTY };
    gates_in_list(list.iter().map(String::as_str))
}

impl WorldScene {
    /// Applies the stage's closed gates when the stage changes, and asks for the region unlock screen when the stage
    /// reaches 12 or 8 from above.
    pub(super) fn career_regions(&mut self) {
        let Some((stage, gates)) = self.career.handle.with(|career| {
            let stage = career.stage();
            (stage, stage_gates(bin_barriers(&career.data.gameplay.bins, stage)))
        }) else {
            return;
        };
        let Some(update) = self.career.regions.evaluate(stage, gates) else {
            return;
        };
        // The gates of a race's run stay off; `restore_region_gates` closes the stage's set when the race is over.
        if !self.career.regions.is_lifted() {
            self.apply_gate_change(update.change);
        }
        if let Some(stage) = update.unlock {
            self.career.requests.push_back(CareerRequest::RegionUnlock { stage });
        }
    }

    /// A race begins: the stage's closed gates go off, so that only the race's own walls stand. The race list of
    /// `Barriers` is switched on after this call, so a list that names a gate (the challenge series) still closes it.
    pub(in crate::scenes::world) fn lift_region_gates(&mut self) {
        let lifted = self.career.regions.lift();
        for group in lifted.numbers().into_iter().filter_map(group_of) {
            self.set_scenery_group(group, false);
        }
        if !lifted.is_empty() {
            log::info!("race: {} region gate(s) lifted for the run", lifted.numbers().len());
            self.refresh_road_barriers();
        }
    }

    /// The race is over, left or restarted: the gates the stage closes come back. Called after the race's own walls
    /// are put back, so a gate group the race switched ends up closed.
    pub(in crate::scenes::world) fn restore_region_gates(&mut self) {
        let closed = self.career.regions.restore();
        for group in closed.numbers().into_iter().filter_map(group_of) {
            self.set_scenery_group(group, true);
        }
        if !closed.is_empty() {
            self.refresh_road_barriers();
        }
    }

    /// Closed gates go on the scenery-group set and opened gates come off it. A gate's group number is its gate number.
    fn apply_gate_change(&mut self, change: GateChange) {
        // A gate's scenery group is not its number: gate 10 is group 624 (`regions::gates::GATES`).
        for group in change.closed.numbers().into_iter().filter_map(group_of) {
            self.set_scenery_group(group, true);
        }
        for group in change.opened.numbers().into_iter().filter_map(group_of) {
            self.set_scenery_group(group, false);
        }
        self.refresh_road_barriers();
    }
}

#[cfg(test)]
#[path = "regions_tests.rs"]
mod tests;
