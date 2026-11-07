//! The `ids` and `remanifest` commands of the importer (`docs/modding/ride6-bikes-batch.md`). `main.rs` hands the arguments
//! after the command word to [`ids`] and [`remanifest`].
//!
//! ```text
//! ids <vehicle id>... | --file <list>          the car id, garage name and brand of each vehicle, one tab-separated line each
//! remanifest <bike folder> <mod folder> --physics <file or dump> [--tyre-tables <dir>] [--car <ID>] [--name ..]
//!            [--manufacturer ..] [--sound-dir <dir>]
//!                                              write cars/<id>.toml again from built.json and the physics data
//! contacts <source or bike folder> <mod folder> --physics <dump>
//!                                              write files/CARS/<ID>/RIDER/contacts.json of each bike (no meshes)
//! cameras <source or bike folder> <mod folder> --physics <dump>
//!                                              write the [bike.camera] tables of each bike's cars/<id>.toml (no meshes)
//! geometry <source> <mod folder> [--max-texture N] [--threads N] [--only <text>]
//!                                              build the car files and livery choices of imported bikes again
//! import-all <source> <mod folder> --physics <dump> [--tyre-tables <dir>] [--sound-dir <dir>] [--max-texture N] [--threads N]
//!                                              import every bike of the exporter's output (what the native mod does)
//! ```

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result};
use clap::Parser;

use crate::bake::Options;
use crate::batch;
use crate::bike_table;
use crate::camera;
use crate::manifest::{self, Tables};
use crate::manifest_sound;
use crate::naming;
use crate::partfile::PartTable;
use crate::regeometry;
use crate::rider::contacts as rider_contacts;
use crate::summary::Summary;

/// `ids`: the names the importer gives a set of bikes. The ids are unique within the set ([`naming::assign`]), so the whole
/// list goes in one call.
pub fn ids(args: &[String]) -> Result<()> {
    let mut vehicles: Vec<String> = Vec::new();
    let mut at = 0;
    while at < args.len() {
        if args[at] != "--file" {
            vehicles.push(args[at].clone());
            at += 1;
            continue;
        }
        let path = args.get(at + 1).context("--file needs a value")?;
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        vehicles.extend(text.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_owned));
        at += 2;
    }
    let ids = naming::assign(&vehicles);
    for (vehicle, id) in vehicles.iter().zip(ids) {
        println!("{vehicle}\t{id}\t{}\t{}", naming::display_name(vehicle), naming::manufacturer(vehicle));
    }
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Write a bike's cars/<id>.toml again from its built.json and the physics data (the meshes are not read)"
)]
struct Remanifest {
    /// The bike's folder: `parts.json` and `built.json` (the importer wrote it beside the part table).
    bike: PathBuf,
    /// The mod folder: its `cars/<id>.toml` is replaced; the car's files are not touched.
    mod_dir: PathBuf,
    /// The car's folder name under CARS/ (default: the rule of `ids`, for the bike alone).
    #[arg(long)]
    car: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    manufacturer: Option<String>,
    #[arg(long)]
    physics: PathBuf,
    #[arg(long)]
    tyre_tables: Option<PathBuf>,
    #[arg(long)]
    sound_dir: Option<PathBuf>,
}

/// `remanifest`: the manifest of a bike converted again from the physics data, without the meshes.
pub fn remanifest(args: &[String]) -> Result<()> {
    let args = Remanifest::try_parse_from(std::iter::once("remanifest".to_owned()).chain(args.iter().cloned()))?;
    let table = PartTable::read(&args.bike)?;
    let summary = Summary::read(&args.bike)?;
    anyhow::ensure!(
        summary.vehicle() == table.vehicle,
        "built.json is of {}, parts.json of {}",
        summary.vehicle(),
        table.vehicle
    );
    let built = summary.into_built();
    let car = args.car.clone().unwrap_or_else(|| naming::default_id(&table.vehicle));
    let name = args.name.clone().unwrap_or_else(|| naming::display_name(&table.vehicle));
    let manufacturer = args.manufacturer.clone().unwrap_or_else(|| naming::manufacturer(&table.vehicle));
    let bike = bike_table::read(&args.physics, args.tyre_tables.as_deref(), &table.vehicle, &built)?;
    let sound = match &args.sound_dir {
        Some(dir) => Some(manifest_sound::read_tables(dir, &car)?),
        None => None,
    };
    let tables = Tables { bike: &bike, sound: sound.as_deref() };
    manifest::write_manifest(&args.mod_dir, &car, (&name, &manufacturer), &built, &tables)?;
    for note in &bike.notes {
        println!("{car}: note: {note}");
    }
    println!("{car}: manifest written from the physics data");
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Write the [bike.camera] tables of already imported bikes' manifests, from RIDE 6's camera containers (the bikes need their RIDER/contacts.json)"
)]
struct Cameras {
    /// One bike's folder (it has a `parts.json`), or the exporter's output with one folder per bike. The car ids of a set
    /// are assigned as the batch import assigns them.
    source: PathBuf,
    /// The mod folder: `cars/<id>.toml` gets its `[bike.camera]` tables; nothing else is touched.
    mod_dir: PathBuf,
    /// The physics dump folder (it holds the camera containers).
    #[arg(long)]
    physics: PathBuf,
}

/// `cameras`: the camera tables of bikes already imported, without converting any mesh.
pub fn cameras(args: &[String]) -> Result<()> {
    let args = Cameras::try_parse_from(std::iter::once("cameras".to_owned()).chain(args.iter().cloned()))?;
    let found = match args.source.join("parts.json").is_file() {
        true => vec![batch::Found { dir: args.source.clone(), vehicle: PartTable::read(&args.source)?.vehicle }],
        false => batch::discover(&args.source)?.0,
    };
    let vehicles: Vec<String> = found.iter().map(|f| f.vehicle.clone()).collect();
    let ids = naming::assign(&vehicles);
    let mut written = 0;
    for (bike, car) in found.iter().zip(&ids) {
        let physics_name =
            PartTable::read(&bike.dir).map_or_else(|_| bike.vehicle.clone(), |table| table.physics_name());
        match camera::apply(&args.mod_dir, car, &args.physics, &physics_name) {
            Ok(()) => written += 1,
            Err(error) => println!("{car}: camera tables not written: {error:#}"),
        }
    }
    println!("camera tables written for {written} of {} bikes", found.len());
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Write the rider's contact points (RIDER/contacts.json) of already imported bikes, from their parts.json"
)]
struct Contacts {
    /// One bike's folder (it has a `parts.json`), or the exporter's output with one folder per bike. The car ids of a set
    /// are assigned as the batch import assigns them.
    source: PathBuf,
    /// The mod folder: `files/CARS/<ID>/RIDER/contacts.json` is written for each bike; nothing else is touched.
    mod_dir: PathBuf,
    /// The physics dump folder (for the correction rows of `RiderAnimationCorrection_DT`).
    #[arg(long)]
    physics: PathBuf,
}

/// `contacts`: the contact points of bikes already imported, without converting any mesh.
pub fn contacts(args: &[String]) -> Result<()> {
    let args = Contacts::try_parse_from(std::iter::once("contacts".to_owned()).chain(args.iter().cloned()))?;
    let (found, _) = match args.source.join("parts.json").is_file() {
        true => {
            let table = PartTable::read(&args.source)?;
            (vec![batch::Found { dir: args.source.clone(), vehicle: table.vehicle }], Vec::new())
        }
        false => batch::discover(&args.source)?,
    };
    let vehicles: Vec<String> = found.iter().map(|f| f.vehicle.clone()).collect();
    let ids = naming::assign(&vehicles);
    let mut written = 0;
    for (bike, car) in found.iter().zip(&ids) {
        let car_dir = args.mod_dir.join("files").join("CARS").join(car.to_ascii_uppercase());
        match rider_contacts::write(&bike.dir, &car_dir, &args.physics) {
            Ok(_) => written += 1,
            Err(error) => println!("{car}: contact points not written: {error:#}"),
        }
    }
    println!("contact points written for {written} of {} bikes", found.len());
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Build the car files and livery choices of already imported bikes again; nothing else of the mod is touched"
)]
struct Geometry {
    /// The exporter's output: one folder per bike, each with `parts.json`, `glb/` and `tex/`.
    source: PathBuf,
    /// The mod folder the bikes were imported into: `files/CARS/<ID>/{GEOMETRY,TEXTURES,BOUNDS}.BIN` and the `[customization]`
    /// table of `cars/<id>.toml` are written.
    mod_dir: PathBuf,
    /// The longest side of a body texture, pixels; wheels get half of it and brake discs a quarter.
    #[arg(long, default_value_t = Options::default().max_texture)]
    max_texture: u32,
    /// Bikes built at the same time (default: half the cores, one to four).
    #[arg(long)]
    threads: Option<usize>,
    /// Only the bikes whose vehicle id contains this text.
    #[arg(long)]
    only: Option<String>,
}

/// `geometry`: the meshes, textures and liveries of imported bikes built again, the manifests kept.
pub fn geometry(args: &[String]) -> Result<()> {
    let args = Geometry::try_parse_from(std::iter::once("geometry".to_owned()).chain(args.iter().cloned()))?;
    let run = regeometry::Run {
        source: &args.source,
        mod_dir: &args.mod_dir,
        max_texture: args.max_texture,
        threads: args.threads.unwrap_or_else(batch::default_threads),
        only: args.only.as_deref(),
    };
    let started = std::time::Instant::now();
    let (written, failed) = regeometry::run(&run)?;
    for (vehicle, reason) in &failed {
        println!("{vehicle}: not written: {reason}");
    }
    println!(
        "car files written for {written} bikes, {} left out, in {:.0} s",
        failed.len(),
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Import every bike of the exporter's output into a mod folder (the native mod's job, without the game)"
)]
struct ImportAll {
    /// The exporter's output: one folder per bike, each with `parts.json`, `glb/` and `tex/`.
    source: PathBuf,
    /// The mod folder to write: `mod.toml`, `cars/` and `files/CARS/`.
    mod_dir: PathBuf,
    /// The physics dump folder.
    #[arg(long)]
    physics: PathBuf,
    #[arg(long)]
    tyre_tables: Option<PathBuf>,
    /// The `sound-bikes` output folder: a bike with tables in it gets its own engine sound.
    #[arg(long)]
    sound_dir: Option<PathBuf>,
    /// The longest side of a body texture, pixels; wheels get half of it and brake discs a quarter.
    #[arg(long, default_value_t = Options::default().max_texture)]
    max_texture: u32,
    /// Bikes imported at the same time (default: half the cores, one to four).
    #[arg(long)]
    threads: Option<usize>,
}

/// `import-all`: every bike of a source folder, with a report of the ones left out.
pub fn import_all(args: &[String]) -> Result<()> {
    let args = ImportAll::try_parse_from(std::iter::once("import-all".to_owned()).chain(args.iter().cloned()))?;
    let cancel = AtomicBool::new(false);
    let input = batch::Input {
        source: &args.source,
        physics: &args.physics,
        tyre_tables: args.tyre_tables.as_deref(),
        sound: args.sound_dir.as_deref(),
        max_texture: args.max_texture,
        threads: args.threads.unwrap_or_else(batch::default_threads),
        cancel: &cancel,
    };
    let started = std::time::Instant::now();
    let report = batch::run(&input, &args.mod_dir)?;
    manifest::write_mod_toml(&args.mod_dir)?;
    for (vehicle, reason) in &report.skipped {
        println!("{vehicle}: left out: {reason}");
    }
    println!(
        "{} bikes imported, {} left out, in {:.0} s",
        report.imported.len(),
        report.skipped.len(),
        started.elapsed().as_secs_f32()
    );
    Ok(())
}
