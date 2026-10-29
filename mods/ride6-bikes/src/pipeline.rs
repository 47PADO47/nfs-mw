//! One bike, end to end: the exporter's folder and the physics dump in, the car's files and manifest out. The command line
//! (`main.rs`) and the native mod ([`crate::batch`]) both call [`import_bike`].

use std::path::Path;

use anyhow::Result;

use crate::bake::Options;
use crate::bike_table;
use crate::build::build;
use crate::manifest::{self, Tables};
use crate::manifest_sound;
use crate::naming;
use crate::partfile::PartTable;
use crate::rider::contacts;
use crate::summary::Summary;

/// What to import and where it goes.
pub struct Request<'a> {
    /// The exporter's folder of the bike: `parts.json`, `glb/`, `tex/`.
    pub bike: &'a Path,
    /// The mod folder (or staging folder) to write: `files/CARS/<ID>/`, `cars/<id>.toml`.
    pub mod_dir: &'a Path,
    /// The car's folder name.
    pub car: &'a str,
    /// The garage name and the brand; `None` takes them from the vehicle id ([`naming`]).
    pub name: Option<&'a str>,
    pub manufacturer: Option<&'a str>,
    pub options: Options,
    /// The bike's resolved physics file, or the dump folder.
    pub physics: &'a Path,
    pub tyre_tables: Option<&'a Path>,
    /// The `sound` command's output folder; its tables join the manifest when it has this car's.
    pub sound_dir: Option<&'a Path>,
    /// Whether a sound folder without this car's tables is an error (the command line) or just a car without sound (a batch).
    pub sound_required: bool,
    /// Whether to write `mod.toml` when the folder has none (a batch writes it once itself).
    pub write_mod_toml: bool,
}

/// What an import did, for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub vehicle: String,
    /// What the physics conversion wants the user to know.
    pub notes: Vec<String>,
    /// Sound files copied, when a sound folder was joined.
    pub sound_files: Option<usize>,
    /// The size summary and the front-end split, as the command line prints them.
    pub summary: String,
    pub front_end: String,
}

/// Import one bike. The car's files are written only when everything succeeded up to the manifest.
pub fn import_bike(request: &Request<'_>) -> Result<Outcome> {
    let table = PartTable::read(request.bike)?;
    let car = request.car;
    let name = request.name.map_or_else(|| naming::display_name(&table.vehicle), str::to_owned);
    let manufacturer = request.manufacturer.map_or_else(|| naming::manufacturer(&table.vehicle), str::to_owned);
    let built = build(request.bike, car, request.options)?;
    if let Err(error) = Summary::of(&table.vehicle, &built).write(request.bike) {
        log::warn!("{car}: built.json is not written: {error:#}");
    }
    let bike = bike_table::read(request.physics, request.tyre_tables, &table.physics_name(), &built)?;
    let sound_tables = match request.sound_dir {
        Some(dir) if request.sound_required || sound_available(dir, car) => {
            Some(manifest_sound::read_tables(dir, car)?)
        }
        _ => None,
    };
    let tables = Tables { bike: &bike, sound: sound_tables.as_deref() };
    manifest::write_car(request.mod_dir, car, (&name, &manufacturer), &built, &tables)?;
    let mut notes = bike.notes;
    let car_dir = request.mod_dir.join("files").join("CARS").join(car.to_ascii_uppercase());
    // The rider's seat, grips and pegs are the bike's own sockets; a bike without them is imported without rider contacts.
    match contacts::write(request.bike, &car_dir, request.physics) {
        Ok(true) => {}
        Ok(false) => notes.push("the rider has no correction row (RiderAnimationCorrection_DT is not in the physics dump, or has no row for this vehicle)".to_owned()),
        Err(error) => notes.push(format!("the rider's contact points are not written: {error:#}")),
    }
    if request.write_mod_toml {
        manifest::write_mod_toml(request.mod_dir)?;
    }
    let sound_files = match request.sound_dir {
        Some(dir) if sound_tables.is_some() => Some(manifest_sound::copy_files(dir, request.mod_dir, car)?),
        _ => None,
    };
    let summary = format!(
        "{} triangles in the body, bounds half dimensions {:?} m, front hub {:?} r {:.3}, rear hub {:?} r {:.3}, \
         {} bytes of geometry, {} of textures",
        built.triangles,
        built.half,
        built.front.hub,
        built.front.radius,
        built.rear.hub,
        built.rear.radius,
        built.geometry.len(),
        built.textures.len()
    );
    Ok(Outcome { vehicle: table.vehicle, notes, sound_files, summary, front_end: built.front_end.to_string() })
}

/// Whether `dir` holds the `sound` command's tables for `car`.
fn sound_available(dir: &Path, car: &str) -> bool {
    dir.join("cars").join(format!("{}.sound.toml", car.to_ascii_lowercase())).is_file()
}
