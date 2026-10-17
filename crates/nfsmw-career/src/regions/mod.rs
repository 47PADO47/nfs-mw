//! Closed regions: the six gate groups, which are closed at which stage, when that is re-evaluated, the map variants
//! and the region unlock screen (`docs/specs/career-regions.md`).
//!
//! - [`gates`]: [`GateSet`], [`closed_gates`], [`apply_stage`], the `Barriers` entry filter and the runtime
//!   [`RegionGates`] (world creation, the deferred stage change, a race on top, `done`).
//! - [`map`]: the minimap and world map variant of a stage and the race `Region` attribute.
//! - [`unlock`]: [`region_unlock_screen`] for stages 12 and 8 and the screen input state machine.

pub mod gates;
pub mod map;
pub mod unlock;

pub use gates::{
    BarrierEntry, GateChange, GateSet, RegionGates, apply_stage, challenge_gates, closed_gates, gate_number,
    gates_in_list, group_state, instance_blocks, parse_barrier_entry,
};
pub use map::{MapVariant, quick_race_region_matches, region_index};
pub use unlock::{RegionPicture, UnlockEvent, UnlockInput, UnlockScreen, UnlockState, region_unlock_screen};
