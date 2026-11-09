//! What the shop screens and the shop's prices read from the vehicle options (`docs/modding/ride6-bike-upgrades.md`, "In the shop").
//!
//! The shop's screens (`frontend/garage_ui`) list the slots of a vehicle that has **components** from this model, so a bike shows
//! its own rims, exhausts, brake discs, bar pads and upgrade ladders by their own names, a slot with nothing to choose is not
//! listed, a component's price and rules are the data's, and a change a rule refuses is refused with the rule's words. A vehicle
//! without components (every car of the 2005 game) keeps the shop's fixed categories: its [`VehicleOptions`] is the same
//! categories (`options/build.rs`), and `Row::Part` and `Row::Performance` read their lists the same way, so nothing differs.

use std::cell::RefCell;

use nfsmw_data::car::customization::Category;
use nfsmw_data::car::mods::components;
use nfsmw_data::car::physics::{PerformanceCategory, PerformancePackage};

use super::{Inputs, Slot, Store, VehicleOptions};
use crate::garage::{Car, Model, Selection};

/// What [`VehicleOptions::of_selection`] reads: the same inputs give the same options, so a screen that asks for them many times
/// a frame (a list of choices asks for each choice) builds them once.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    folder: String,
    parts: nfsmw_data::car::customization::Selection,
    performance: PerformancePackage,
    color: Option<[u8; 3]>,
    /// The catalogue the options were read from: where it is, and how many choices it lists (it is read once and then shared).
    catalog: (usize, usize),
    levels: [u8; 7],
}

/// The options built last, newest first: the cart's and the installed car's are asked for in turn.
const CACHED: usize = 4;

thread_local! {
    static RECENT: RefCell<Vec<(Key, VehicleOptions)>> = const { RefCell::new(Vec::new()) };
}

fn key(car: &Car, selection: &Selection) -> Key {
    let catalog = car.parts.as_deref();
    Key {
        folder: car.folder.clone(),
        parts: selection.parts.clone(),
        performance: selection.performance,
        color: selection.color,
        catalog: catalog.map_or((0, 0), |catalog| {
            (std::ptr::from_ref(catalog) as usize, catalog.choices.iter().map(Vec::len).sum())
        }),
        levels: std::array::from_fn(|at| car.performance.categories[at].max_level),
    }
}

/// The shop section a slot is listed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShopSection {
    Parts,
    Performance,
    Visual,
}

/// Whether `car` has components of its own that list slots (a RIDE 6 bike): its shop is driven by them.
pub(crate) fn has_components(car: &Car) -> bool {
    components::of(&car.folder).is_some_and(|own| !own.vehicle.slots.is_empty())
}

impl VehicleOptions {
    /// The options of `car` with `selection` fitted, for a car that has components (`None` for the others: their categories are
    /// the shop's own rows).
    pub(crate) fn of_selection(car: &Car, selection: &Selection) -> Option<Self> {
        if !has_components(car) {
            return None;
        }
        let key = key(car, selection);
        if let Some(found) = RECENT.with_borrow(|recent| recent.iter().find(|(known, _)| *known == key).cloned()) {
            return Some(found.1);
        }
        let options = Self::build(&Inputs {
            folder: &car.folder,
            name: &car.name,
            catalog: car.parts.as_deref(),
            parts: &selection.parts,
            color: selection.color,
            paintable: car.paintable,
            original_color: car.original_color,
            performance: Some((&car.performance, selection.performance)),
        });
        RECENT.with_borrow_mut(|recent| {
            recent.insert(0, (key, options.clone()));
            recent.truncate(CACHED);
        });
        Some(options)
    }

    /// The options of the car the shop shows, with the cart fitted (`None` for a car without components).
    pub(crate) fn of_draft(model: &Model) -> Option<Self> {
        Self::of_selection(model.selected()?, &model.draft)
    }

    /// The slot that keeps its choice in the model category `category`.
    pub(crate) fn of_category(&self, category: Category) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.store == Store::Category(category))
    }

    /// The slot that keeps its choice as the level of `category`.
    pub(crate) fn of_level(&self, category: PerformanceCategory) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.store == Store::Level(category))
    }

    /// The slots the shop lists in `section`, with their position in [`VehicleOptions::slots`], the ones with a choice besides the
    /// stock one only (an empty slot is not listed).
    pub(crate) fn listed(&self, section: ShopSection) -> impl Iterator<Item = (usize, &Slot)> {
        self.slots
            .iter()
            .enumerate()
            .filter(move |(_, slot)| slot.choices.len() > 1 && section_of(slot) == Some(section))
    }
}

/// The section of the shop a slot belongs to: the garage's own model categories are the Parts (body kit, spoiler, hood, roof scoop,
/// rims), its levels the Performance, and a slot only the components know goes by its `group`: `performance` there, `appearance`
/// or `visual` in the Visual section, any other (`looks`, `parts`...) in Parts. `None` for the categories that keep their own
/// pages (paint, rim paint, window tint, vinyls).
fn section_of(slot: &Slot) -> Option<ShopSection> {
    match slot.store {
        Store::Category(category) => Category::MODELS.contains(&category).then_some(ShopSection::Parts),
        Store::Level(_) => Some(ShopSection::Performance),
        Store::Session => Some(match slot.group.as_str() {
            "performance" => ShopSection::Performance,
            "appearance" | "visual" => ShopSection::Visual,
            _ => ShopSection::Parts,
        }),
    }
}
