//! `ride6-rider-ragdoll <RiderGP_PhysicsAsset.json> <RiderBP.json> <output folder>`: writes the rider's `ragdoll.json`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Converts the RIDE 6 rider's physics asset and the fall configurations of his blueprint into the ragdoll the game's rider
/// falls with.
#[derive(Parser)]
struct Args {
    /// The rider's physics asset, as the package reader's `json` mode writes it.
    physics_asset: PathBuf,
    /// The rider blueprint (`RiderBP`), the same way.
    blueprint: PathBuf,
    /// Where to write `ragdoll.json` (usually mods/ride6-bikes/files/RIDER).
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = ride6_bikes::rider_ragdoll::import(&args.physics_asset, &args.blueprint, &args.output)?;
    println!("{summary:?}");
    Ok(())
}
