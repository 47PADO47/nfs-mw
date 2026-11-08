//! `ride6-track-import <pack folder> <mod folder> [--max-texture 512]`: turn a RIDE 6 track pack (the exporter's output,
//! `docs/formats/ride6-track-pack.md`) into the files of a second world: `files/WORLDS/<ID>/` and `worlds/`.
//!
//! The pack is made from the user's own install by `tools/ride6-tracks/export.py`; nothing of the game is stored in this
//! repository.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Convert a RIDE 6 track pack into a second world of NFS: Most Wanted")]
struct Args {
    /// The folder the exporter wrote: `pack.json`, `ground.r6m`, `tex/`.
    pack: PathBuf,
    /// The mod folder to write: `files/WORLDS/<ID>/` and `worlds/`.
    mod_dir: PathBuf,
    /// The longest side of a ground picture, pixels.
    #[arg(long, default_value_t = 512)]
    max_texture: u32,
    /// After the import, cast a ray down at every sample of the lap and count the surfaces and the heights met.
    #[arg(long)]
    check: bool,
}

fn run(args: &Args) -> Result<()> {
    let started = std::time::Instant::now();
    let report = ride6_tracks::import::import(&args.pack, &args.mod_dir, args.max_texture, &AtomicBool::new(false))?;
    println!(
        "world {}: {} cells, {} triangles in {} solids, {} pictures, {:.1} MB of sections in {:.1} s",
        report.world,
        report.cells,
        report.triangles,
        report.solids,
        report.pictures,
        report.bytes as f64 / 1e6,
        started.elapsed().as_secs_f32()
    );
    println!("collision: {} triangles, {} too wide for a strip", report.collision_triangles, report.collision_skipped);
    for (picture, reason) in &report.missing_pictures {
        println!("picture {picture} is drawn flat: {reason}");
    }
    if args.check {
        check(args)?;
    }
    Ok(())
}

/// The lap against the converted collision (`ride6_tracks::check`).
fn check(args: &Args) -> Result<()> {
    let pack = ride6_tracks::pack::read(&args.pack)?;
    let world = ride6_tracks::check::load_world(&args.mod_dir, &pack)?;
    let report = ride6_tracks::check::along_the_lap(&world, &pack);
    println!(
        "lap: {} samples, {} with no ground, worst height difference {:.2} m, {} more than 1 m off",
        report.samples, report.missed, report.worst_height, report.off_height
    );
    for (surface, count) in &report.surfaces {
        println!("  {surface}: {count}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ride6-track-import: {error:#}");
            ExitCode::FAILURE
        }
    }
}
