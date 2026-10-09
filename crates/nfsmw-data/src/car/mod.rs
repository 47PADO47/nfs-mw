//! Loading one car from the install and assembling it from the parts database:
//! stock (or preset) parts, wheels and brakes at the four corners, paint and texture
//! swaps. Spec: docs/specs/car-assembly.md.

mod assemble;
mod ecar;
pub mod exhaust;
mod paint;
mod parts;
pub mod physics;
mod stock;
mod swaps;
mod tables;
mod textures;
mod wheels;

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use game_install::GameDir;
use glam::Mat4;

pub use assemble::Placement;
pub use ecar::WheelSetup;
pub use paint::Paint;
pub use parts::is_stock_part;
pub use stock::{Slots, preset_parts, stock_parts};
pub use swaps::TextureSwaps;
pub use tables::{CarTables, LAYOUT, slot};
pub use wheels::{Corner, WheelModel, WheelPose};

use crate::read_unwrapped;

/// A car ready to draw: solids placed in car space (+x forward, +y left, +z up).
pub struct CarModel {
    /// Car folder.
    pub name: String,
    /// The car type it was assembled as, if the car tables know the folder.
    pub car_type: Option<String>,
    /// The solids used, by name hash.
    pub solids: HashMap<u32, Solid>,
    pub placements: Vec<Placement>,
    /// Textures the solids use (after swaps), by name hash.
    pub textures: HashMap<u32, Texture>,
    pub swaps: TextureSwaps,
    pub paint: Option<Paint>,
    /// Height of the car origin above the floor (tyres resting on it), 0 when unknown.
    pub floor_height: f32,
    /// The four wheel and brake mounts, to pose a moving car; with the `ecar` wheel record.
    pub corners: Option<[Corner; 4]>,
    pub wheel_setup: Option<WheelSetup>,
}

#[derive(Debug, Clone)]
pub struct LoadOptions {
    /// Level of detail, 'A' (highest) to 'E'.
    pub lod: char,
    /// Show every solid of the LOD at the origin instead of assembling the car (debugging).
    pub all_parts: bool,
    /// Build a `PresetRides` car (e.g. `CE_GTRSTREET`) instead of the stock car.
    pub preset: Option<String>,
}

/// Car folders that have a model.
pub fn list(dir: &GameDir) -> Vec<String> {
    dir.subdirectories("CARS").into_iter().filter(|car| dir.exists(&format!("CARS/{car}/GEOMETRY.BIN"))).collect()
}

/// The folder named `wanted`, ignoring case; failing that, the only folder that starts with it.
pub fn pick_folder<'a>(cars: &'a [String], wanted: &str) -> Option<&'a str> {
    if let Some(exact) = cars.iter().find(|c| c.eq_ignore_ascii_case(wanted)) {
        return Some(exact);
    }
    let lower = wanted.to_ascii_lowercase();
    let mut starts = cars.iter().filter(|c| c.to_ascii_lowercase().starts_with(&lower));
    match (starts.next(), starts.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

/// Shared part folders (aftermarket wheels, brakes, spoilers, roof scoops, plates).
fn is_shared_part_folder(name: &str) -> bool {
    ["WHEELS", "BRAKES", "PLATES", "ROOF"].contains(&name) || name.starts_with("SPOILER")
}

/// Load and assemble `car` (a folder under `CARS/`).
pub fn load(dir: &GameDir, car: &str, options: &LoadOptions) -> Result<CarModel> {
    let lod = usize::from(options.lod.to_ascii_uppercase() as u8).wrapping_sub(usize::from(b'A'));
    if lod > 4 {
        bail!("LOD must be A to E, not {:?}", options.lod);
    }
    let own = read_solids(dir, car)?;
    let tables = if options.all_parts { None } else { Some(CarTables::load(dir)?) };
    let car_type = tables.as_ref().and_then(|t| t.car_type_for_folder(car));
    let (Some(t), Some(car_type)) = (tables.as_ref(), car_type) else {
        if !options.all_parts {
            log::warn!("{car} is not in the car tables; showing its KIT00 solids unassembled");
        }
        return unassembled(dir, car, own, options);
    };

    let slots = match &options.preset {
        Some(name) => preset_parts(t, car_type, t.preset(name).with_context(|| format!("no preset named {name}"))?),
        None => stock_parts(t, car_type),
    };
    let mut pool: HashMap<u32, Solid> = own.into_iter().map(|s| (s.name_hash, s)).collect();
    let wanted: Vec<u32> = slots[..slot::MODELS].iter().flatten().filter_map(|p| t.parts.model_hash(p, lod)).collect();
    if wanted.iter().any(|h| !pool.contains_key(h)) {
        for folder in dir.subdirectories("CARS").into_iter().filter(|f| is_shared_part_folder(f)) {
            for s in read_solids(dir, &folder)? {
                pool.entry(s.name_hash).or_insert(s);
            }
        }
    }

    let setup = WheelSetup::read(&t.attributes, &car_type.base_model_name);
    if setup.is_none() {
        log::warn!("{car}: no ecar record for {}; wheels are not placed", car_type.base_model_name);
    }
    let (placements, corners) = assemble::assemble(t, &slots, &pool, setup.as_ref(), lod);
    let used: HashSet<u32> = placements.iter().map(|p| p.solid).collect();
    pool.retain(|h, _| used.contains(h));

    let swaps = TextureSwaps::for_car(&car_type.base_model_name);
    let paint = slots[slot::BASE_PAINT].map(|p| Paint::of(t, &p));
    let mut textures = textures::load(dir, car, &wanted_textures(&pool, &swaps))?;
    if let (true, Some(paint)) = (car_type.skinnable, &paint) {
        for name in paint::skin_placeholders(&car_type.base_model_name) {
            textures.insert(bstring_hash(&name), paint.texture(&name));
        }
    }
    log::info!(
        "{car}: {} as {}{}: {} placements of {} solids, {} textures, paint {}",
        options.preset.as_deref().unwrap_or("stock"),
        car_type.type_name,
        if car_type.skinnable { " (skinnable)" } else { "" },
        placements.len(),
        pool.len(),
        textures.len(),
        paint.as_ref().map_or("none".into(), |p| format!("{} {}", p.name, p.hex())),
    );
    Ok(CarModel {
        name: car.to_owned(),
        car_type: Some(car_type.type_name.clone()),
        solids: pool,
        placements,
        textures,
        swaps,
        paint,
        floor_height: setup.as_ref().map_or(0.0, wheels::floor_height),
        corners,
        wheel_setup: setup,
    })
}

/// Every texture the solids may draw: placeholders, their replacements and the left caliper.
fn wanted_textures(solids: &HashMap<u32, Solid>, swaps: &TextureSwaps) -> HashSet<u32> {
    let mut wanted: HashSet<u32> = solids
        .values()
        .flat_map(|s| s.groups.iter().filter_map(|g| g.diffuse_texture(s)))
        .flat_map(|h| [h, swaps.resolve(h)])
        .collect();
    wanted.insert(bstring_hash(swaps::BRAKE_LEFT));
    wanted
}

/// The solids of `CARS/<folder>/GEOMETRY.BIN`.
fn read_solids(dir: &GameDir, folder: &str) -> Result<Vec<Solid>> {
    let path = format!("CARS/{folder}/GEOMETRY.BIN");
    if !dir.exists(&path) {
        bail!("no car named {folder:?} (try `nfsmw list-cars`)");
    }
    blackbox_solid::read_solids(&read_unwrapped(dir, &path)?).with_context(|| format!("parsing {path}"))
}

/// Without the parts database: the solids of one LOD at the origin, filtered by name.
fn unassembled(dir: &GameDir, car: &str, solids: Vec<Solid>, options: &LoadOptions) -> Result<CarModel> {
    let solids: HashMap<u32, Solid> = solids
        .into_iter()
        .filter(|s| is_stock_part(&s.name, options.lod.to_ascii_uppercase(), options.all_parts))
        .map(|s| (s.name_hash, s))
        .collect();
    if solids.is_empty() {
        bail!("{car} has no solids at LOD {}", options.lod);
    }
    let swaps = TextureSwaps::default();
    let placements = solids
        .keys()
        .map(|&solid| Placement { solid, transform: Mat4::IDENTITY, left_brake: false, slot: usize::MAX, corner: None })
        .collect();
    let textures = textures::load(dir, car, &wanted_textures(&solids, &swaps))?;
    Ok(CarModel {
        name: car.to_owned(),
        car_type: None,
        solids,
        placements,
        textures,
        swaps,
        paint: None,
        floor_height: 0.0,
        corners: None,
        wheel_setup: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cars() -> Vec<String> {
        ["BMWM3", "BMWM3GTR", "CARRERAGT", "CAMARO", "CORVETTE"].iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn exact_beats_prefix_and_prefix_must_be_unique() {
        let cars = cars();
        assert_eq!(pick_folder(&cars, "bmwm3"), Some("BMWM3"), "exact match even though BMWM3GTR also starts with it");
        assert_eq!(pick_folder(&cars, "carr"), Some("CARRERAGT"));
        assert_eq!(pick_folder(&cars, "ca"), None, "camaro and carreragt");
        assert_eq!(pick_folder(&cars, "zzz"), None);
    }
}
