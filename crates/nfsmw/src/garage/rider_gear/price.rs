//! What the rider's gear costs in a bike's shop.
//!
//! The mod's catalogue (`RIDER/GEAR/gear.json`) carries no prices and neither does the 2005 price table (its entities are car
//! parts), so every item of a part costs the part's **default price** below: a helmet is the dearest of the small items, a suit the
//! dearest of all, and the rider's own gear is free. These are **[ours]**, chosen to sit near the first tier of the car shop's
//! visual parts (a vinyl, a paint), not read from any game. An item the profile already owns costs nothing to wear again
//! (`rider_gear::owns`). The prices do not go through the career's unlock table: gear is open from the first visit to a bike's shop.

use super::catalogue::Part;
use super::choice::Choice;

/// The price of any item of `part` that is not the rider's own gear.
pub const fn default_price(part: Part) -> u32 {
    match part {
        Part::Body => 500,
        Part::Helmet => 1500,
        Part::Suit => 3000,
        Part::Gloves => 600,
        Part::Boots => 900,
    }
}

/// What wearing `id` instead of what is worn costs: nothing for the rider's own gear and for an item `owned` says is the
/// player's, the part's default price for the rest.
pub fn cost(part: Part, id: &str, owned: impl Fn(Part, &str) -> bool) -> u32 {
    match id.is_empty() || owned(part, id) {
        true => 0,
        false => default_price(part),
    }
}

/// The parts `selected` wears differently from `installed`, each with what the change costs.
pub fn changes(installed: &Choice, selected: &Choice, owned: impl Fn(Part, &str) -> bool + Copy) -> Vec<(Part, u32)> {
    Part::ALL
        .into_iter()
        .filter(|part| installed.get(*part) != selected.get(*part))
        .map(|part| (part, cost(part, selected.get(part), owned)))
        .collect()
}
