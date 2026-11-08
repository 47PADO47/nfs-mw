//! The rider's gear: which helmet, suit, gloves, boots and body the player's rider wears on a bike
//! (`docs/specs/ride6-rider-gear.md`).
//!
//! A mod lists what it offers in `RIDER/GEAR/gear.json` ([`catalogue`]); the player picks in the garage's Visual category
//! (`frontend/garage_ui/rider_gear.rs`) and the choice is kept beside the profile's save ([`choice`]). The setting `rider_gear`
//! turns the whole feature off, and then the rider wears his own gear. The rider (`scenes/world/drive/rider`) reads the choice
//! when it loads and again when [`generation`] has moved; the garage's preview shows a choice before it is kept ([`set_preview`]).

pub(crate) mod catalogue;
pub(crate) mod choice;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use catalogue::{Catalogue, Entry, Part};
pub(crate) use choice::Choice;
pub(crate) use state::{
    catalogue as loaded, commit, committed, current, enabled, generation, reload, scan, set_enabled, set_preview,
};
