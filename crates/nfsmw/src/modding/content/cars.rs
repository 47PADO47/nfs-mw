//! The cars of the mods: each `cars/<id>.toml` is read and checked, and the cars are registered against the car table
//! of the install (`nfsmw_data::car::mods`). A car that fails any check is logged and left out; the game goes on.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use blackbox_modding::content::{CARS_FOLDER, CarManifest, parse_car};
use game_install::{GameDir, Provider};
use nfsmw_data::car::CarTables;
use nfsmw_data::car::mods::custom;
use nfsmw_data::car::mods::{self as car_mods, CarSpec, CarStats, EngineLayer, EngineSound};
use nfsmw_data::pursuit::HEAT_LEVELS;

use super::ModDir;

/// Register the cars that the mods declare and publish them: at start-up for every loaded mod, and again for one mod
/// that added cars later. A car that is already published is skipped, so a repeated call adds only what is new.
/// Without any new car request the tables are not read at all. Returns how many cars were added.
pub(super) fn register(dir: &GameDir, mods: &[ModDir]) -> usize {
    let published = car_mods::published();
    let is_new = |spec: &CarSpec| !published.iter().any(|car| car.folder.eq_ignore_ascii_case(&spec.folder));
    let requests: Vec<(CarSpec, Option<custom::Customization>)> =
        mods.iter().flat_map(|m| specs_of(dir, m)).filter(|(spec, _)| is_new(spec)).collect();
    let mut customizations: HashMap<String, custom::Customization> = HashMap::new();
    let mut specs = Vec::with_capacity(requests.len());
    for (spec, own) in requests {
        if let Some(own) = own {
            customizations.entry(spec.folder.clone()).or_insert(own);
        }
        specs.push(spec);
    }
    if specs.is_empty() {
        return 0;
    }
    let tables = match CarTables::load(dir) {
        Ok(tables) => tables,
        Err(error) => {
            log::warn!("mod cars: the car tables cannot be read, so {} car(s) are not added: {error:#}", specs.len());
            return 0;
        }
    };
    let (cars, rejected) = car_mods::register(&tables.types, &specs);
    for refused in &rejected {
        log::warn!("mod {}: car {} is not added: {}", refused.owner, refused.folder, refused.reason);
    }
    for car in &cars {
        log::info!("mod {}: car {} added", car.owner, car.folder);
        if let Some(own) = customizations.remove(&car.folder) {
            custom::publish(&car.folder, own);
        }
    }
    let added = cars.len();
    car_mods::publish(cars);
    added
}

/// The car requests of one mod: its `cars/*.toml` files, in name order.
fn specs_of(dir: &GameDir, m: &ModDir) -> Vec<(CarSpec, Option<custom::Customization>)> {
    manifest_files(&m.dir.join(CARS_FOLDER))
        .into_iter()
        .filter_map(|path| match read_spec(dir, &m.id, &path) {
            Ok(spec) => Some(spec),
            Err(reason) => {
                log::warn!("mod {}: {} is not read: {reason}", m.id, path.display());
                None
            }
        })
        .collect()
}

/// The `.toml` files directly in `folder`, sorted by name. A missing folder has none.
fn manifest_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("toml")))
        .collect();
    files.sort();
    files
}

/// Read and check one manifest, and turn it into a request the car table can check.
fn read_spec(dir: &GameDir, owner: &str, path: &Path) -> Result<(CarSpec, Option<custom::Customization>), String> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or("the file name is not text")?;
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest = parse_car(stem, &text).map_err(|error| error.to_string())?;
    let cop_heat = manifest.role.is_cop().then(|| cop_levels(&manifest));
    let stats = CarStats::from(&manifest.stats);
    let geometry = format!("CARS/{}/GEOMETRY.BIN", manifest.id);
    let geometry_in_mod = matches!(dir.provider(&geometry), Some(Provider::Overlay(id)) if id == owner);
    let own = manifest.customization.as_ref().map(customization_of);
    let spec = CarSpec {
        folder: manifest.id,
        owner: owner.to_owned(),
        name: manifest.name,
        manufacturer: manifest.manufacturer,
        player: manifest.role.is_player(),
        cop_heat,
        geometry_in_mod,
        stats,
        engine_sound: manifest.engine_sound.as_ref().map(engine_sound_of),
    };
    Ok((spec, own))
}

/// The manifest's `[customization]` as the game's own customization of a mod car.
fn customization_of(table: &blackbox_modding::content::Customization) -> custom::Customization {
    use blackbox_modding::content::Finish;
    let part =
        |part: &blackbox_modding::content::Part| custom::Part { name: part.name.clone(), model: part.model.clone() };
    custom::Customization {
        paints: table
            .paints
            .iter()
            .map(|paint| custom::Paint {
                name: paint.name.clone(),
                color: paint.color,
                finish: match paint.finish {
                    Finish::Gloss => custom::Finish::Gloss,
                    Finish::Metallic => custom::Finish::Metallic,
                    Finish::Chrome => custom::Finish::Chrome,
                },
            })
            .collect(),
        default_paint: table.default_paint,
        rim_paints: table
            .rim_paints
            .iter()
            .map(|rim| custom::RimPaint { name: rim.name.clone(), color: rim.color })
            .collect(),
        tints: table
            .tints
            .iter()
            .map(|tint| custom::Tint { name: tint.name.clone(), color: tint.color, opacity: tint.opacity })
            .collect(),
        rims: table.rims.iter().map(part).collect(),
        spoilers: table.spoilers.iter().map(part).collect(),
        roof_scoops: table.roof_scoops.iter().map(part).collect(),
        vinyls: table
            .vinyls
            .iter()
            .map(|vinyl| custom::Vinyl {
                name: vinyl.name.clone(),
                bitmap: vinyl.bitmap.clone(),
                mask: vinyl.mask.clone(),
                channels: vinyl.channels,
            })
            .collect(),
        vinyl_colors: table
            .vinyl_colors
            .iter()
            .map(|color| custom::VinylColor { name: color.name.clone(), color: color.color })
            .collect(),
    }
}

/// The manifest's `[engine_sound]` as the game's engine loops.
fn engine_sound_of(sound: &blackbox_modding::content::EngineSound) -> EngineSound {
    EngineSound {
        idle_rpm: sound.idle_rpm,
        redline_rpm: sound.redline_rpm,
        layers: sound.layers.iter().map(|layer| EngineLayer { file: layer.file.clone(), rpm: layer.rpm }).collect(),
    }
}

/// The levels a cop car is in: its list, or every level of the game when the list was left out. A level the game does
/// not have is refused by the car table (`car_mods::register`).
fn cop_levels(manifest: &CarManifest) -> Vec<u32> {
    if manifest.heat.is_empty() {
        return (1..=HEAT_LEVELS).collect();
    }
    manifest.heat.clone()
}
