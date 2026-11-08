//! The entries of the shop's sections for a vehicle with components (ADR 0014, `garage/options`): its own slots, by their own names,
//! in the order its data gives, a slot with nothing to choose left out. A vehicle without components gets `None` here and keeps
//! the fixed categories of the original shop (`choices.rs`), which are its options' categories too.

use super::{Hardware, Item, Model, Page, Part, Row, Section, hardware_texture};
use crate::garage::options::{ShopSection, Slot, Store, VehicleOptions};

/// The picture of a model category's entry, as the original lists it.
fn category_icon(category: Part) -> u32 {
    match category {
        Part::Body => 0x028c_24f6,
        Part::Spoiler => 0xbb03_4ea6,
        Part::Wheels => 0x0294_d2a3,
        Part::Hood => 0x028f_7092,
        Part::Roof => 0x7916_5861,
        _ => 0x3e31_ba56,
    }
}

/// The picture of the entry of a slot only the components know: the Parts and Visual sections' own.
const SESSION_ICON: u32 = 0x3e31_ba56;
const PERFORMANCE_ICON: u32 = 0x3987_d054;

/// The entries of `section` for the vehicle `model` shows; `None` for a vehicle without components and for the Visual section
/// (which keeps its own pages, with the components' slots added by [`visual`]).
pub(super) fn section(section: Section, model: &Model, backroom: bool) -> Option<Vec<Item>> {
    let car = model.selected()?;
    let options = VehicleOptions::of_draft(model)?;
    // The fixed hero cars of the original have no parts to sell; a mod car is never one.
    if car.hero {
        return Some(Vec::new());
    }
    match section {
        Section::Parts => Some(parts(&options, backroom)),
        Section::Performance => Some(performance(&options, model, backroom)),
        Section::Visual => None,
    }
}

fn parts(options: &VehicleOptions, backroom: bool) -> Vec<Item> {
    options
        .listed(ShopSection::Parts)
        // A component slot no model category holds has no backroom marker to pay with.
        .filter(|(_, slot)| !backroom || slot.store != Store::Session)
        .map(|(at, slot)| entry(at, slot, backroom))
        .collect()
}

fn performance(options: &VehicleOptions, model: &Model, backroom: bool) -> Vec<Item> {
    options
        .listed(ShopSection::Performance)
        .filter(|(_, slot)| !backroom || matches!(slot.store, Store::Level(_)))
        .map(|(at, slot)| {
            let mut item = entry(at, slot, backroom);
            // The induction picture follows the car's: a turbo or a supercharger.
            if let Store::Level(Hardware::Induction) = slot.store
                && model.selected().is_some_and(|car| !car.performance_details.turbo)
            {
                item.texture = match backroom {
                    true => 0x6300_71e4,
                    false => 0x9360_3dfb,
                };
            }
            item
        })
        .collect()
}

/// The entry of one slot: a page of its choices, on the row that keeps them.
fn entry(at: usize, slot: &Slot, backroom: bool) -> Item {
    let (row, texture) = match slot.store {
        Store::Category(category) => (Row::Part(category), category_icon(category)),
        Store::Level(category) => (
            match backroom {
                true => Row::Junkman(category),
                false => Row::Performance(category),
            },
            hardware_texture(category, backroom),
        ),
        Store::Session => (
            Row::Slot(u8::try_from(at).unwrap_or(u8::MAX)),
            match slot.group.as_str() {
                "performance" => PERFORMANCE_ICON,
                _ => SESSION_ICON,
            },
        ),
    };
    Item::page(&slot.label, texture, Page::Choices { row, backroom }, true)
}

/// The Visual section's entries only the components know (a slot of group `appearance` or `visual`), after the pages every vehicle has.
pub(super) fn visual(model: &Model) -> Vec<Item> {
    let Some(options) = VehicleOptions::of_draft(model) else { return Vec::new() };
    options.listed(ShopSection::Visual).map(|(at, slot)| entry(at, slot, false)).collect()
}
