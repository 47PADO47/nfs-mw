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
    /// Riding style of every bike type: 0 knee out, 1 plain, 2 old school (the inside foot leaves the peg of a supermoto). Without
    /// it each type plays its own: the supermoto the old-school style, every other type the first.
    #[arg(long)]
    style: Option<u8>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = ride6_bikes::rider::import(&args.input, &args.output, args.style)?;
    println!("{summary:?}");
    Ok(())
}
