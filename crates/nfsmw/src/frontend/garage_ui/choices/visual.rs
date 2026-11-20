//! The Visual section's entries: the pages every vehicle has (paint, vinyls, rim paint, window tint, decals, numbers, plate,
//! gauges), the rider's gear of a bike in a career shop, and the component slots only a vehicle's components know.

use super::{Item, Model, Page, Part, Row, slots};
use nfsmw_data::car::customization::Catalog;

pub(super) fn visual(model: &Model, backroom: bool) -> Vec<Item> {
    // A vehicle with components lists only what it has to choose: a page with nothing behind it is left out.
    let own = model.selected().is_some_and(crate::garage::options::has_components);
    let catalog = model.selected().and_then(|car| car.parts.as_deref());
    let has = |part: Part| !own || catalog.is_some_and(|catalog| !catalog.choices[part as usize].is_empty());
    let mut result = vec![Item::page(
        "Paint",
        match backroom {
            true => 0x0db8_9e17,
            false => 0xa3b7_6154,
        },
        Page::Paint { color: 0, backroom },
        Row::Color(0).enabled(model),
    )];
    if has(Part::Vinyl) {
        result.push(Item::page(
            "Vinyls",
            match backroom {
                true => 0xd35f_04c0,
                false => 0x5577_8e5a,
            },
            Page::VinylGroups { backroom },
            true,
        ));
    }
    if !backroom {
        // The rider's gear comes right after Paint and Vinyls: on a bike it is what sets the Visual category apart.
        result.extend(rider_pages(model));
        result.extend(slots::visual(model));
        if model
            .selected()
            .filter(|car| !car.hero)
            .and_then(|car| car.parts.as_ref())
            .is_some_and(|catalog| !catalog.choices[Part::Wheels as usize].is_empty())
            && has(Part::RimPaint)
        {
            result.push(Item::page(
                "Rim Paint",
                0xd223_f84a,
                Page::Paint { color: 1, backroom },
                Row::Color(1).enabled(model),
            ));
        }
        if has(Part::WindowTint) {
            result.push(Item::page(
                "Window Tint",
                0x3f23_165c,
                Page::Choices { row: Row::Part(Part::WindowTint), backroom },
                true,
            ));
        }
    }
    let decals = !own || catalog.is_some_and(|catalog| catalog.decals.iter().any(|zone| !zone.choices.is_empty()));
    if decals {
        result.push(Item::page(
            "Decals",
            match backroom {
                true => 0xa913_5927,
                false => 0xda1d_ae54,
            },
            Page::DecalZones { backroom },
            true,
        ));
    }
    if !backroom {
        let numbers = Row::Number(0).enabled(model);
        if numbers || !own {
            result.push(Item::page("Numbers", 0x45a1_c644, Page::Numbers, numbers));
        }
        result.extend(plate_pages(model.selected().and_then(|car| car.parts.as_deref())));
    }
    let gauges = !own || catalog.is_some_and(|catalog| catalog.gauge_skins.iter().any(|skin| *skin != 0));
    if gauges {
        result.push(Item::page(
            "Custom Gauges",
            match backroom {
                true => 0x8ba6_02fc,
                false => 0x028f_88bc,
            },
            Page::Choices { row: Row::Gauge, backroom },
            true,
        ));
    }
    result
}

/// The licence plate entries of the Visual category, from the car's catalogue: "License Plate" only for a car whose manifest has
/// a `[customization.plate]`, and "Plate Design" only when that plate also offers designs. A stock car lists neither. The pages
/// have no icon of their own: they take the Numbers one.
pub(super) fn plate_pages(catalog: Option<&Catalog>) -> Vec<Item> {
    let Some(catalog) = catalog.filter(|catalog| catalog.plate) else { return Vec::new() };
    let mut result = vec![Item::page("License Plate", 0x45a1_c644, Page::Plate, true)];
    if !catalog.plate_designs.is_empty() {
        result.push(Item::page("Plate Design", 0x45a1_c644, Page::PlateDesign, true));
    }
    result
}

/// The rider's gear entries of the Visual category (`docs/specs/ride6-rider-gear.md`): one page for each part a mod offers
/// items for, on a bike (a mod car of `kind = "bike"`) in a career shop, while the setting `rider_gear` is on. A car, My Cars, the
/// free roam garage and the backroom list none: the gear is for sale there only.
pub(super) fn rider_pages(model: &Model) -> Vec<Item> {
    let bike = model.selected().is_some_and(|car| nfsmw_data::car::mods::bike::is_bike(&car.folder));
    gear_pages(
        shop_sells_gear(bike, model.shop, model.backroom, crate::garage::rider_gear::enabled()),
        crate::garage::rider_gear::loaded().as_deref(),
    )
}

/// Whether the gear is for sale: only a bike in a career shop (not My Cars, not the free roam garage, not a car, not the backroom)
/// while the setting is on. Anywhere else the gear is the developer tools' (`gear`, the F11 panel).
pub(super) fn shop_sells_gear(bike: bool, shop: bool, backroom: bool, setting: bool) -> bool {
    bike && shop && !backroom && setting
}

/// The pages for `catalogue`, when `offered`: the parts with something to choose from, each with the helmet icon.
pub(super) fn gear_pages(offered: bool, catalogue: Option<&crate::garage::rider_gear::Catalogue>) -> Vec<Item> {
    let Some(catalogue) = catalogue.filter(|_| offered) else { return Vec::new() };
    crate::garage::rider_gear::Part::ALL
        .into_iter()
        .filter(|part| catalogue.entries(*part).len() > 1)
        .map(|part| Item::page(part.label(), crate::ui::rider_icon::TEXTURE, Page::Rider { part }, true))
        .collect()
}
