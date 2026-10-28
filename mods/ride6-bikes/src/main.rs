//! `ride6-bike-import <exports> <mod folder>`: turn the glTF exports of a RIDE 6 bike into the files of a mod car.
//!
//! The exports are the user's own, made from their own install; nothing of the game is stored in this repository.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use ride6_bikes::build::{Exports, build};
use ride6_bikes::manifest;

#[derive(Parser)]
#[command(about = "Convert the glTF exports of a RIDE 6 bike into a mod car of NFS: Most Wanted")]
struct Args {
    /// The folder with the exported `.glb` files (every level of detail: `chassis369.glb`, `chassis369_LOD2.glb`, ...).
    exports: PathBuf,
    /// The mod folder to write: `mod.toml`, `cars/` and `files/CARS/`.
    mod_dir: PathBuf,
    /// The car's folder name under CARS/ (upper case, up to 32 letters, digits, `_` or `-`).
    #[arg(long, default_value = "KTM450")]
    car: String,
    /// The longest side of a texture, pixels.
    #[arg(long, default_value_t = 512)]
    max_texture: u32,
}

fn run(args: &Args) -> Result<()> {
    let exports = Exports::scan(&args.exports)?;
    let built = build(&exports, &args.car, args.max_texture)?;
    manifest::write(&args.mod_dir, &args.car, &built)?;
    println!(
        "{}: {} triangles, bounds half dimensions {:?} m, {} bytes of geometry, {} of textures",
        args.car,
        built.triangles,
        built.half,
        built.geometry.len(),
        built.textures.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
