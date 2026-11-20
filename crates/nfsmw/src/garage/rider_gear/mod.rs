//! The rider's gear: which helmet, suit, gloves, boots and body the player's rider wears on a bike
//! (`docs/specs/ride6-rider-gear.md`).
//!
//! A mod lists what it offers in `RIDER/GEAR/gear.json` ([`catalogue`]). The player buys gear in a bike's shop, in the Visual
//! category (`frontend/garage_ui/gear_page.rs`, priced by [`price`], part of the shop's cart) or sets it from the developer tools
//! (the `gear` command and the vehicle panel); the choice is kept beside the profile's save ([`choice`]), one for every bike. The
//! setting `rider_gear` turns the whole feature off, and then the rider wears his own gear. The rider
//! (`scenes/world/drive/rider`) reads the choice when it loads and again when [`generation`] has moved; the garage's preview shows
//! a choice before it is kept ([`set_preview`]).

pub(crate) mod catalogue;
pub(crate) mod choice;
pub(crate) mod price;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use catalogue::{Catalogue, Entry, Part, position};
pub(crate) use choice::Choice;
pub(crate) use state::{
    catalogue as loaded, commit, committed, current, enabled, generation, kept, owns, purchase, reload, scan,
    set_enabled, set_preview, validate_change,
};
