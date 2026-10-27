//! The parts of a mod car, from the car's own solids (docs/modding/car-customization.md "The files of a car").
//!
//! The parts database of the install lists the parts of the cars the game ships and nothing of a mod's. A mod car
//! gets the parts its assembly needs, named after its folder like the solids the 2012 mod writes: the body, whose
//! model at each level of detail is `<FOLDER>_KIT00_BODY_<A..D>`, the wheel, `<FOLDER>_KIT00_FRONT_TIRE_<A..D>` (the
//! one solid every corner draws; the game turns it for the left side) and the driver, `<FOLDER>_KIT00_DRIVER_<A..D>`,
//! which the game draws in the cockpit while the car is driven. A car whose `GEOMETRY.BIN` has no driver solid draws no
//! driver: a part names its models, and a slot whose model is not among the loaded solids draws nothing. Every other
//! slot stays empty: the car has no brakes apart from the wheel solid's own, no hood, spoiler, damage overlay or decal
//! part, and its paint, window tint, rims and vinyls come from its `[customization]` table (`mods/custom`).

use anyhow::{Context, Result};
use blackbox_carparts::{CarTypeInfo, NewPart, PartsDb};
use blackbox_hash::bstring_hash;

use crate::car::tables::{LAYOUT, slot};

/// The levels of detail a mod car's solids may have, as the letters of their names. `E` is the coarsest and absent.
const LEVELS: [char; 4] = ['A', 'B', 'C', 'D'];

/// The hash of each level of the solid `<base>_<letter>`; the levels a car lacks name no solid.
fn level_hashes(base: &str) -> [u32; 5] {
    let mut hashes = [u32::MAX; 5];
    for (hash, letter) in hashes.iter_mut().zip(LEVELS) {
        *hash = bstring_hash(&format!("{base}_{letter}"));
    }
    hashes
}

/// Add the body, wheel and driver parts of each car of `cars` (the mod cars' table entries) to `parts`. A car whose parts
/// do not fit is left out whole, and the log says why; the others are added.
pub fn extend(parts: &mut PartsDb, cars: &[CarTypeInfo]) {
    for car in cars {
        if let Err(e) = add(parts, car) {
            log::warn!("{}: its parts are not in the parts database: {e:#}", car.type_name);
        }
    }
}

fn add(parts: &mut PartsDb, car: &CarTypeInfo) -> Result<()> {
    let folder = &car.type_name;
    let body = level_hashes(&format!("{folder}_KIT00_BODY"));
    let wheel = level_hashes(&format!("{folder}_KIT00_FRONT_TIRE"));
    let body_name = format!("{folder}_BODY");
    let wheel_name = format!("{folder}_FRONT_WHEEL");
    let driver = level_hashes(&format!("{folder}_KIT00_DRIVER"));
    let driver_name = format!("{folder}_DRIVER");
    let news = [
        NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::BODY].part_id,
            name: &body_name,
            upgrade_level: 0,
            models: &body,
            attributes: &[],
        },
        NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::FRONT_WHEEL].part_id,
            name: &wheel_name,
            upgrade_level: 0,
            models: &wheel,
            attributes: &[],
        },
        NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::DRIVER].part_id,
            name: &driver_name,
            upgrade_level: 0,
            models: &driver,
            attributes: &[],
        },
    ];
    parts.push_parts(&news).with_context(|| format!("adding the parts of {folder}"))?;
    Ok(())
}
