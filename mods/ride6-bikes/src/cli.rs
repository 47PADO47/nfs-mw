//! The `ids` and `remanifest` commands of the importer (`docs/modding/ride6-bikes-batch.md`). `main.rs` hands the arguments
//! after the command word to [`ids`] and [`remanifest`].
//!
//! ```text
//! ids <vehicle id>... | --file <list>          the car id, garage name and brand of each vehicle, one tab-separated line each
//! remanifest <bike folder> <mod folder> --physics <file or dump> [--tyre-tables <dir>] [--car <ID>] [--name ..]
//!            [--manufacturer ..] [--sound-dir <dir>]
//!                                              write cars/<id>.toml again from built.json and the physics data
//! ```

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::bike_table;
use crate::manifest::{self, Tables};
use crate::manifest_sound;
use crate::naming;
use crate::partfile::PartTable;
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
