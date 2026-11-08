//! `ride6-rider-import <export folder> <output folder>`: converts the exported RIDE 6 rider into the runtime pack.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Converts the exported RIDE 6 rider into the files the game's rider runtime reads.
#[derive(Parser)]
struct Args {
    /// The folder the export tool wrote (riding.r6ra, skeleton.json, bike_skeleton.json, meshes/, textures/); with `--merge`
    /// only skeleton.json and the animations (`*.r6ra`, `acl/`).
    input: PathBuf,
    /// Where to write (usually mods/ride6-bikes/files/RIDER).
    output: PathBuf,
    /// Add the animations of the export to the rider already in the output folder and write the graphs again, instead of
    /// converting the whole rider.
    #[arg(long)]
    merge: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = match args.merge {
        true => ride6_bikes::rider::merge(&args.input, &args.output)?,
        false => ride6_bikes::rider::import(&args.input, &args.output)?,
    };
    println!("{summary:?}");
    Ok(())
}
