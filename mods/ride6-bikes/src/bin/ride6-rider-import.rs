//! `ride6-rider-import <export folder> <output folder>`: converts the exported RIDE 6 rider into the runtime pack.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Converts the exported RIDE 6 rider into the files the game's rider runtime reads.
#[derive(Parser)]
struct Args {
    /// The folder the export tool wrote (riding.r6ra, skeleton.json, bike_skeleton.json, meshes/, textures/).
    input: PathBuf,
    /// Where to write (usually mods/ride6-bikes/files/RIDER).
    output: PathBuf,
    /// Riding style: 0 knee-out lean clips, 1 plain, 2 old school.
    #[arg(long, default_value_t = 1)]
    style: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = ride6_bikes::rider::import(&args.input, &args.output, args.style)?;
    println!("{summary:?}");
    Ok(())
}
