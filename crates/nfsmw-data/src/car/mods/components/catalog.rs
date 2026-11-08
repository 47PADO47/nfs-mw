//! The garage's lists for a car with components: its choices in the model categories and the levels of its upgrade ladders.

use blackbox_components::Effect;
use game_install::GameDir;

use super::Components;
use super::select::{category_of, choice_hash, performance_of};
use crate::car::customization::{Catalog, Category, Choice};
use crate::car::mods::{CarStats, bike};
use crate::car::physics::PerformanceCatalog;

/// Whether every part a component shows has its pack in the install: a component whose parts are gone is not offered. A part
/// in the car's own files is not checked here (the car is read for its own solids anyway), and a solid a pack lacks draws
/// nothing ([`super::parts`]).
fn present(dir: &GameDir, components: &Components, id: &str) -> bool {
    let Some(component) = components.catalog.get(id) else { return false };
    let shown = component.effects.iter().flat_map(|effect| match effect {
        Effect::Part { show, .. } => show.as_slice(),
        _ => &[],
    });
    shown
        .filter_map(|part| components.parts.get(part))
        .filter_map(|binding| binding.pack.as_ref())
        .all(|pack| dir.exists(&format!("{pack}/GEOMETRY.BIN")))
}

/// Add the choices of the car's slots to the garage catalogue `catalog`: each slot's accepted components but its default (the
/// stock part is the empty choice), in the order the car lists them, in the category [`category_of`] names.
pub fn offer(dir: &GameDir, folder: &str, components: &Components, catalog: &mut Catalog) {
    for slot in &components.vehicle.slots {
        let Some(category) = category_of(&slot.id) else { continue };
        for offer in slot.accepts.iter().filter(|offer| Some(&offer.component) != slot.default.as_ref()) {
            if !present(dir, components, &offer.component) {
                log::warn!("{folder}: {} is not offered: a part of it is missing from the install", offer.component);
                continue;
            }
            let Some(component) = components.catalog.get(&offer.component) else { continue };
            let hash = choice_hash(folder, &component.id);
            let label = component.display_name().to_owned();
            catalog.choices[category as usize].push(Choice { hash, label, language_hash: None, carbon: false });
            catalog.part_groups.insert(hash, 0);
            if category == Category::Wheels {
                catalog.rim_metadata.insert(hash, (0, 0));
            }
        }
    }
}

/// What the shop offers a mod car: a bike takes only the upgrades it has a ladder for (the levels of its performance slots),
/// every other mod car what its stats give ([`PerformanceCatalog::from_stats`]).
pub fn performance(folder: &str, stats: &CarStats) -> PerformanceCatalog {
    let mut catalog = PerformanceCatalog::from_stats(stats);
    if !bike::is_bike(folder) {
        return catalog;
    }
    for option in &mut catalog.categories {
        option.max_level = 0;
    }
    let Some(components) = super::of(folder) else { return catalog };
    for slot in &components.vehicle.slots {
        let Some(category) = performance_of(&slot.id) else { continue };
        catalog.categories[category as usize].max_level = components.max_tier(&slot.id);
    }
    catalog
}
