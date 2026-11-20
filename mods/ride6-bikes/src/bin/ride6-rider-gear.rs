//! `ride6-rider-gear <catalogue folder> <output folder>`: converts the gear catalogue of the user's RIDE 6 export into the files
//! the game's rider reads (`docs/formats/ride6-rider-gear.md`).

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Converts the exported RIDE 6 rider gear (helmets, suits, gloves, boots) into the runtime files of the rider's `GEAR` folder.
#[derive(Parser)]
struct Args {
    /// The folder `tools/ride6-exporter/gear.py` wrote (catalogue.json, meshes/, tex/).
    catalogue: PathBuf,
    /// Where to write (usually mods/ride6-bikes/files/RIDER/GEAR).
    output: PathBuf,
    /// The longest side of a picture kept, pixels: the larger mip levels are dropped.
    #[arg(long, default_value_t = ride6_bikes::rider::gear::DEFAULT_MAX_TEXTURE)]
    max_texture: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = ride6_bikes::rider::gear::import(&args.catalogue, &args.output, args.max_texture)?;
    println!("{summary:#?}");
    Ok(())
}
