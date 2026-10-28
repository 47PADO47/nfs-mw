//! The parts of a mod car, from the car's own solids (docs/modding/car-customization.md "The files of a car").
//!
//! The parts database of the install lists the parts of the cars the game ships and nothing of a mod's. A mod car
//! gets the parts its assembly needs, named after its folder like the solids the 2012 mod writes: the body, whose
//! model at each level of detail is `<FOLDER>_KIT00_BODY_<A..D>`, the wheel, `<FOLDER>_KIT00_FRONT_TIRE_<A..D>` (the
//! one solid every corner draws; the game turns it for the left side) and the driver, `<FOLDER>_KIT00_DRIVER_<A..D>`,
//! which the game draws in the cockpit while the car is driven. A bike has two different wheels, so it also gets the rear
//! wheel, `<FOLDER>_KIT00_REAR_TIRE_<A..D>`, in the rear wheel slot, and its front end that steers, `<FOLDER>_KIT00_FORK_<A..D>`,
//! in the front brake slot, and its handlebar group, `<FOLDER>_KIT00_BARS_<A..D>`, in the rear brake slot (a bike draws no
//! brakes of a car's kind; each placement is posed like a brake: the fork's steers and travels with the front wheel, the
//! bars' only steers, neither spins; docs/modding/bikes.md). A car whose `GEOMETRY.BIN`
//! has no driver solid draws no driver: a part names its models, and a slot whose model is not among the loaded solids
//! draws nothing. Every other slot stays empty: the car has no brakes apart from the wheel solid's own, no hood, spoiler,
//! damage overlay or decal part, and its paint, window tint, rims and vinyls come from its `[customization]` table
//! (`mods/custom`).

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
    let rear = level_hashes(&format!("{folder}_KIT00_REAR_TIRE"));
    let rear_name = format!("{folder}_REAR_WHEEL");
    let fork = level_hashes(&format!("{folder}_KIT00_FORK"));
    let fork_name = format!("{folder}_FORK");
    let bars = level_hashes(&format!("{folder}_KIT00_BARS"));
    let bars_name = format!("{folder}_BARS");
    let mut news = vec![
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
    // A bike has two different wheels: its rear tyre is a part of its own, in the rear wheel slot. Its fork, the front end
    // that steers, is in the front brake slot; a bike whose `GEOMETRY.BIN` has no fork solid draws none.
    if super::bike::is_bike(folder) {
        news.push(NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::REAR_WHEEL].part_id,
            name: &rear_name,
            upgrade_level: 0,
            models: &rear,
            attributes: &[],
        });
        news.push(NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::FRONT_BRAKE].part_id,
            name: &fork_name,
            upgrade_level: 0,
            models: &fork,
            attributes: &[],
        });
        news.push(NewPart {
            type_name_hash: car.type_name_hash,
            part_id: LAYOUT.slots[slot::REAR_BRAKE].part_id,
            name: &bars_name,
            upgrade_level: 0,
            models: &bars,
            attributes: &[],
        });
    }
    parts.push_parts(&news).with_context(|| format!("adding the parts of {folder}"))?;
    Ok(())
}
