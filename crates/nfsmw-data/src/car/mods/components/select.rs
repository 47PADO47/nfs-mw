//! What the garage keeps, as a component selection, and the resolution of it.
//!
//! The garage keeps two things for a car: the model choices of its [`Selection`] (a hash per category, kept in the profile's
//! `my-cars.toml`) and the performance package (a level per category, kept in the save, exactly as for every other car). A
//! car with components maps its slots onto these existing places, so nothing new is saved and every screen that edits them
//! (the garage, the F6 panel, `view-car --part`) edits the components:
//!
//! | Slot of the car | Where the garage keeps it |
//! |---|---|
//! | `exhaust` | the Spoiler category: a hash of the choice |
//! | `bar_pad` | the Hood category |
//! | `rims` | the Rims category |
//! | `brake_discs` | the Body kit category |
//! | `perf_engine`, `perf_chassis`, `perf_brakes` | the level of the Engine, Suspension and Brakes performance category |
//!
//! A choice is told apart by a hash of the car's folder and the component's id, so a saved car keeps its choice when the mod
//! reorders its lists and loses it when the mod renames the component.

use blackbox_components::{Catalog, Pick, ResolvedVehicle, Selection as Picks, Slot};
use blackbox_hash::bstring_hash;

use super::Components;
use crate::car::customization::{Category, Selection};
use crate::car::physics::{PerformanceCategory, PerformancePackage};

/// The garage category a slot's choices are listed in; `None` for a slot the garage has no place for.
pub fn category_of(slot: &str) -> Option<Category> {
    match slot {
        "exhaust" => Some(Category::Spoiler),
        "bar_pad" => Some(Category::Hood),
        "rims" => Some(Category::Wheels),
        "brake_discs" => Some(Category::Body),
        _ => None,
    }
}

/// The performance category whose level a slot's tier is; `None` for a slot that is not an upgrade ladder.
pub fn performance_of(slot: &str) -> Option<PerformanceCategory> {
    match slot {
        "perf_engine" => Some(PerformanceCategory::Engine),
        "perf_chassis" => Some(PerformanceCategory::Chassis),
        "perf_brakes" => Some(PerformanceCategory::Brakes),
        _ => None,
    }
}

/// The hash that stands for a component of the car in `folder` in the garage's catalogue and in a saved selection.
pub fn choice_hash(folder: &str, component: &str) -> u32 {
    bstring_hash(&format!("{}.COMPONENT.{}", folder.to_ascii_uppercase(), component.to_ascii_uppercase()))
}

/// The component of `slot` whose tier is the highest not above `level`; `None` for level 0 (the slot's default).
fn ladder_pick(slot: &Slot, catalog: &Catalog, level: u8) -> Option<Pick> {
    if level == 0 {
        return None;
    }
    let tier = |id: &str| catalog.get(id).map_or(0, |component| component.tier);
    let best = slot
        .accepts
        .iter()
        .map(|offer| offer.component.as_str())
        .filter(|id| (1..=u32::from(level)).contains(&tier(id)))
        .max_by_key(|id| tier(id))?;
    Some(Pick::of(best))
}

/// The components `selection` and `package` install on the car in `folder`. A choice the car does not list is ignored: a
/// saved car whose mod changed falls back to the stock part.
pub fn picks(folder: &str, components: &Components, selection: &Selection, package: PerformancePackage) -> Picks {
    let mut picks = Picks::default();
    for slot in &components.vehicle.slots {
        if let Some(category) = category_of(&slot.id)
            && let Some(hash) = selection.parts[category as usize]
            && let Some(offer) = slot.accepts.iter().find(|offer| choice_hash(folder, &offer.component) == hash)
        {
            picks.set(&slot.id, Pick::of(&offer.component));
        }
        if let Some(category) = performance_of(&slot.id)
            && let Some(pick) = ladder_pick(slot, &components.catalog, package.get(category))
        {
            picks.set(&slot.id, pick);
        }
    }
    picks
}

/// The car with the components of `selection` and `package` installed. When they do not fit together (a rule of the mod) the
/// log says so and the car is stock.
pub fn resolve(
    folder: &str,
    components: &Components,
    selection: &Selection,
    package: PerformancePackage,
) -> Option<ResolvedVehicle> {
    let picks = picks(folder, components, selection, package);
    match components.vehicle.resolve(&components.catalog, &picks) {
        Ok(resolved) => Some(resolved),
        Err(problems) => {
            log::warn!("{folder}: the parts picked do not fit together ({problems:?}); the car is stock");
            components.vehicle.resolve(&components.catalog, &Picks::default()).ok()
        }
    }
}
