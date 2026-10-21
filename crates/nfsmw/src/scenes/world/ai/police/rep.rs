//! The rep points and the cost to state of a chase (`docs/specs/ai-pursuit-heat.md` sections 5.1, 5.2 and 5.4): the
//! running totals the career banks when the chase ends and the HUD shows while it runs. Child of `police`, so it reads
//! the police state directly. The rates and the destruction points come from the track's `RepTable`.

use nfsmw_data::pursuit::RepTable;

use super::Police;

/// Seconds of chase per step of the normal rep points: the code steps every 10 s (spec 5.1).
const STEP_SECS: f32 = 10.0;
/// The highest destruction multiplier (spec 5.1).
const MAX_MULTIPLIER: u32 = 3;
/// Cost to state of one cop car destroyed (spec 5.4).
const COST_COP_DESTROYED: u32 = 5000;
/// Cost to state of one cop car damaged (spec 5.4).
const COST_COP_DAMAGED: u32 = 250;
/// Cost to state of one roadblock deployed (spec 5.4).
const COST_ROADBLOCK: u32 = 500;
/// Cost to state of one roadblock prop deployed (`mNumSpikeStripsDeployed` counts every prop, spec 5.4).
const COST_PROP: u32 = 250;
/// Cost to state of one helicopter spawned (spec 5.4).
const COST_HELICOPTER: u32 = 2000;

/// The running totals of one chase. [`RepState::reset`] clears them when a chase starts and keeps the table.
#[derive(Debug, Default)]
pub(super) struct RepState {
    /// The rates and destruction points of the track.
    table: RepTable,
    /// Cop cars wrecked in the chase.
    cops_destroyed: u32,
    /// Normal rep points: the chase's time in sight.
    normal: u32,
    /// Destruction rep points.
    destroyed: u32,
    /// Cost to state.
    cost: u32,
    /// Cop cars damaged, as last counted.
    cops_damaged: u32,
    /// The destruction multiplier: 1 at the first kill of a window, up to `MAX_MULTIPLIER` while the window runs.
    multiplier: u32,
    /// Seconds left in the destruction window (`DestroyCopBonusTime` from the last kill).
    window_secs: f32,
}

impl RepState {
    /// Clears the totals for a new chase, keeping the table.
    pub(super) fn reset(&mut self) {
        let table = std::mem::take(&mut self.table);
        *self = Self { table, ..Self::default() };
    }

    /// The rep points banked so far: normal and destruction.
    fn points(&self) -> u32 {
        self.normal.saturating_add(self.destroyed)
    }

    /// Runs the destruction window down by `dt` seconds.
    fn tick_window(&mut self, dt: f32) {
        self.window_secs = (self.window_secs - dt).max(0.0);
    }

    /// A cop car was wrecked: counted, costed, and its points times the multiplier. A kill while the window runs raises
    /// the multiplier by one, up to the maximum; otherwise it is x1. The window restarts. Returns the points added.
    fn wreck(&mut self, points: u32, window_secs: f32) -> u32 {
        self.multiplier = match self.window_secs > 0.0 {
            true => (self.multiplier + 1).min(MAX_MULTIPLIER),
            false => 1,
        };
        self.window_secs = window_secs;
        self.cops_destroyed = self.cops_destroyed.saturating_add(1);
        self.cost = self.cost.saturating_add(COST_COP_DESTROYED);
        let gained = points.saturating_mul(self.multiplier);
        self.destroyed = self.destroyed.saturating_add(gained);
        gained
    }

    /// The chase's time moved from `before` to `after` seconds in sight: the normal points of each 10 s step crossed.
    fn time_in_sight(&mut self, before: f32, after: f32, per_step: u32) {
        let steps = steps_crossed(before, after);
        self.normal = self.normal.saturating_add(steps.saturating_mul(per_step));
    }

    /// The cop cars damaged so far is `count`: each one not yet counted costs `COST_COP_DAMAGED`.
    fn damaged_to(&mut self, count: u32) {
        let new = count.saturating_sub(self.cops_damaged);
        self.cost = self.cost.saturating_add(new.saturating_mul(COST_COP_DAMAGED));
        self.cops_damaged = count;
    }

    /// Adds `amount` to the cost to state.
    fn add_cost(&mut self, amount: u32) {
        self.cost = self.cost.saturating_add(amount);
    }
}

/// The 10 s steps of chase crossed going from `before` to `after` seconds: the multiples of the step in (before, after].
/// Nothing when time does not move forward or a value is not a number.
pub(super) fn steps_crossed(before: f32, after: f32) -> u32 {
    if after.is_nan() || before.is_nan() || after <= before {
        return 0;
    }
    let steps = (after / STEP_SECS).floor() - (before / STEP_SECS).floor();
    steps.max(0.0) as u32
}

impl Police {
    /// Sets the rep rates and destruction points of the track (read once per track).
    pub fn set_rep_table(&mut self, table: RepTable) {
        self.rep.table = table;
    }

    /// A cop car of type `car` (lower-case name) was wrecked in the chase. Only a chase counts it: its rep points at the
    /// chase's heat, times the multiplier, and its cost to state.
    pub fn record_wreck(&mut self, car: &str) {
        let Some(level) = self.level() else { return };
        let points = self.rep.table.destroy_points(car, level);
        let window = self.rep.table.rate(level).destroy_bonus_secs;
        self.rep.wreck(points, window);
    }

    /// The chase's clock moved from `before` by `dt` seconds (called as the pursuit time advances). Runs the destruction
    /// window down, and banks the normal points while the cops see the player and the player is not busted.
    pub(super) fn tick_rep(&mut self, before: f32, dt: f32, seen: bool, level: u32) {
        self.rep.tick_window(dt);
        if !seen || self.is_busted() {
            return;
        }
        let per_step = self.rep.table.rate(level).per_ten_seconds;
        self.rep.time_in_sight(before, before + dt, per_step);
    }

    /// The cop cars damaged so far in the chase, counted by the world each frame.
    pub fn note_cops_damaged(&mut self, count: u32) {
        self.rep.damaged_to(count);
    }

    /// A roadblock was deployed in the chase: its cost to state.
    pub(super) fn on_roadblock(&mut self) {
        if self.heat.is_none() {
            return;
        }
        self.rep.add_cost(COST_ROADBLOCK);
    }

    /// `count` roadblock props (sawhorses and spike strips alike) were deployed: their cost to state (spec 5.4).
    pub(super) fn on_roadblock_props(&mut self, count: u32) {
        if self.heat.is_none() {
            return;
        }
        self.rep.add_cost(COST_PROP.saturating_mul(count));
    }

    /// A helicopter joined the chase: its cost to state.
    pub(super) fn on_helicopter(&mut self) {
        if self.heat.is_none() {
            return;
        }
        self.rep.add_cost(COST_HELICOPTER);
    }

    /// The cop cars wrecked in the chase.
    pub fn cops_destroyed(&self) -> u32 {
        self.rep.cops_destroyed
    }

    /// The rep points of the chase so far: the pending bounty (normal plus destruction).
    pub fn rep_points(&self) -> u32 {
        self.rep.points()
    }

    /// The normal rep points of the chase so far (the time in sight).
    pub fn rep_normal_points(&self) -> u32 {
        self.rep.normal
    }

    /// The destruction rep points of the chase so far.
    pub fn rep_destroyed_points(&self) -> u32 {
        self.rep.destroyed
    }

    /// The cost to state of the chase so far.
    pub fn cost_to_state(&self) -> u32 {
        self.rep.cost
    }
}

#[cfg(test)]
#[path = "rep_tests.rs"]
mod tests;
