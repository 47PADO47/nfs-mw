//! The rider's gear in the Gameplay options: one row for each part a mod offers items for (Rider Body, Helmet, Suit, Gloves, Boots),
//! listed after the Rider Gear switch, so the gear can be picked without the garage (`docs/specs/ride6-rider-gear.md`, section 5).
//!
//! A row is a value the player cycles with Left and Right: the item's name, the rider's own gear first. The choice is kept at once,
//! beside the profile's save, as the garage's page keeps it (`garage/rider_gear`); a rider on a bike in the world dresses again.
//! The rows are dimmed while the switch is off, and absent while no mod offers gear.

use super::steps::cycle;
use super::{Data, Row, Setting, Title, row};
use crate::garage::rider_gear::{self, Catalogue, Entry, Part};

/// The title of a part's row.
fn title(part: Part) -> Title {
    Title::Key(match part {
        Part::Body => "options-rider-body",
        Part::Helmet => "options-rider-helmet",
        Part::Suit => "options-rider-suit",
        Part::Gloves => "options-rider-gloves",
        Part::Boots => "options-rider-boots",
    })
}

/// The rows of the parts `catalogue` has something to pick for; none without a catalogue.
pub(super) fn rows(catalogue: Option<&Catalogue>) -> Vec<Row> {
    let Some(catalogue) = catalogue else { return Vec::new() };
    Part::ALL
        .into_iter()
        .filter(|part| catalogue.entries(*part).len() > 1)
        .map(|part| row(Setting::Gear(part), title(part)))
        .collect()
}

/// The position of the item `id` in `entries`; the rider's own gear (the first) for an id the list lacks.
fn position(entries: &[Entry], id: &str) -> usize {
    entries.iter().position(|entry| entry.id == id).unwrap_or(0)
}

/// The list of a part and the position of the kept item in it; `None` without a catalogue.
fn kept(part: Part) -> Option<(Vec<Entry>, usize)> {
    let entries = rider_gear::loaded()?.entries(part);
    let at = position(&entries, rider_gear::kept().get(part));
    Some((entries, at))
}

/// What a part's row shows: the name of the kept item.
pub(super) fn data(part: Part) -> Data {
    let Some((entries, at)) = kept(part) else { return Data::Text(String::new()) };
    Data::Text(entries[at].label.clone())
}

/// Keeps the next or previous item of a part for the profile. Returns whether the choice changed.
pub(super) fn step(part: Part, forward: bool) -> bool {
    let Some((entries, at)) = kept(part) else { return false };
    // The item after (`forward`) or before the one kept, wrapping around.
    let next = &entries[cycle(at, entries.len(), forward)];
    if let Err(error) = rider_gear::commit(part, &next.id) {
        log::warn!("the rider's {} is not kept: {error:#}", part.key());
        return false;
    }
    true
}

#[cfg(test)]
#[path = "options_gear_tests.rs"]
mod tests;
