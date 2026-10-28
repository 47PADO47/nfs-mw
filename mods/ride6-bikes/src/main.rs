//! `ride6-bike-import <bike folder> <mod folder> --physics <resolved json>`: turn the exported parts and the physics data of a
//! RIDE 6 bike into the files of a mod car.
//!
//! The bike folder is what the exporter script wrote (`parts.json`, `glb/`, `tex/`; see `docs/modding/ride6-bikes.md`), the
//! physics file the resolved JSON of the physics dump. Both are made from the user's own install; nothing of the game is
//! stored in this repository.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use ride6_bikes::bake::Options;
use ride6_bikes::cli;
use ride6_bikes::naming;
use ride6_bikes::partfile::PartTable;
use ride6_bikes::pipeline::{Request, import_bike};
use ride6_bikes::sound;

#[derive(Parser)]
#[command(about = "Convert the exported parts of a RIDE 6 bike into a mod car of NFS: Most Wanted")]
struct Args {
    /// The folder the exporter wrote: `parts.json`, `glb/` with every level of detail of every part, `tex/`.
    bike: PathBuf,
    /// The mod folder to write: `mod.toml`, `cars/` and `files/CARS/`.
    mod_dir: PathBuf,
    /// The car's folder name under CARS/ (upper case, up to 32 letters, digits, `_` or `-`). Default: derived from the
    /// vehicle id by the rule of `ride6-bike-import ids` (`docs/modding/ride6-bikes-batch.md`).
    #[arg(long)]
    car: Option<String>,
    /// The garage name of the car (default: the vehicle id with spaces).
    #[arg(long)]
    name: Option<String>,
    /// The manufacturer (default: the first word of the vehicle id).
    #[arg(long)]
    manufacturer: Option<String>,
    /// The longest side of a body texture, pixels; wheels get half of it and brake discs a quarter.
    #[arg(long, default_value_t = 2048)]
    max_texture: u32,
    /// The occlusion factor of a fully occluded texel (0 to 1) baked into the colour.
    #[arg(long, default_value_t = 0.3)]
    ao_floor: f32,
    /// Also write normal and specular textures and tangent vertices (effect 3) for the tangent-space shader.
    #[arg(long)]
    normal_maps: bool,
    /// Invert the green channel of the normal maps.
    #[arg(long)]
    flip_green: bool,
    /// The bike's resolved physics data: the `vehicles/<Brand>/<Vehicle>.json` file of the physics dump, or the dump folder
    /// (the file is then found by the vehicle id of the part table). It becomes the manifest's `[bike]` table.
    #[arg(long)]
    physics: PathBuf,
    /// The `Tire` folder of the dump (`resolved/plugin/Physics/Tire`): read for the tyre assets the vehicle file does not link.
    #[arg(long)]
    tyre_tables: Option<PathBuf>,
    /// The folder the `sound` command wrote (its `--out`): its tables join the manifest, its sound files are copied.
    #[arg(long)]
    sound_dir: Option<PathBuf>,
}

fn run(args: &Args) -> Result<()> {
    let table = PartTable::read(&args.bike)?;
    let car = args.car.clone().unwrap_or_else(|| naming::default_id(&table.vehicle));
    let options = Options {
        max_texture: args.max_texture,
        ao_floor: args.ao_floor.clamp(0.0, 1.0),
        normal_maps: args.normal_maps,
        flip_green: args.flip_green,
    };
    let request = Request {
        bike: &args.bike,
        mod_dir: &args.mod_dir,
        car: &car,
        name: args.name.as_deref(),
        manufacturer: args.manufacturer.as_deref(),
        options,
        physics: &args.physics,
        tyre_tables: args.tyre_tables.as_deref(),
        sound_dir: args.sound_dir.as_deref(),
        sound_required: true,
        write_mod_toml: true,
    };
    let outcome = import_bike(&request)?;
    if let Some(copied) = outcome.sound_files {
        println!("{car}: sound tables joined to the manifest, {copied} sound files copied");
    }
    for note in &outcome.notes {
        println!("{car}: note: {note}");
    }
    println!("{car}: {}", outcome.summary);
    println!("{car}: front end (the finest level): {}", outcome.front_end);
    Ok(())
}

/// The sound commands are words in front of their own arguments: `ride6-bike-import sound <options>` builds a bike's
/// sound files and manifest tables, `ride6-bike-import extract-bnk <bank> <folder>` splits a sound bank
/// (`docs/modding/ride6-bikes-sound.md`), `ids` names a set of bikes and `remanifest` converts a bike's physics again
/// (`docs/modding/ride6-bikes-batch.md`). Anything else is the importer proper.
fn sound_command(argv: &[String]) -> Option<Result<()>> {
    let rest = argv.get(2..)?;
    match argv.get(1)?.as_str() {
        "sound" => Some(sound::cli::run(rest)),
        "extract-bnk" => Some(sound::cli::extract_bnk(rest)),
        "ids" => Some(cli::ids(rest)),
        "remanifest" => Some(cli::remanifest(rest)),
        _ => None,
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let result = sound_command(&argv).unwrap_or_else(|| run(&Args::parse()));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
