//! `fairhaven-unit`: makes the section file of one unit (or every unit) of a 2012 install, the way the game does when the camera
//! comes near it, and says what it came to. For measuring and for looking at a unit's file; the game does not need it.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use fairhaven::convert::{Options, Stats, convert_unit};
use fairhaven::install::Install;
use fairhaven::manifest::numbered;

/// Make the file of a Fairhaven unit from the 2012 install.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// The folder of the 2012 install (with `HAWAII/` and `SHADERS.BNDL`).
    install: PathBuf,
    /// The unit number (`TRK_UNIT<n>.BNDL`). Without it, every unit is made.
    unit: Option<u32>,
    /// The longest side of a picture, pixels.
    #[arg(long, default_value_t = 512)]
    max_texture: u32,
    /// How bright the pictures are, percent of their stored colour.
    #[arg(long, default_value_t = fairhaven::config::DEFAULT_BRIGHTNESS)]
    brightness: u32,
    /// Write the file of the unit to this path (one unit only).
    #[arg(long)]
    out: Option<PathBuf>,
    /// List the models each unit lacks and look for them in every bundle of the install (not only the ones the reader opens).
    #[arg(long)]
    find_missing: bool,
}

fn line(unit: u32, stats: &Stats) -> String {
    format!(
        "unit {unit:5}: {:6} ms  {:5} instances + {:4} objects ({} clipped, {} missing, {} too large)  {:5} placements  {:5} solids  {:8} triangles  {:4} pictures ({} flat)  {:7} collision triangles ({} skipped)  {:6.2} MB  (pictures {} ms, collision {} ms, writing {} ms)",
        stats.millis,
        stats.instances,
        stats.objects,
        stats.clipped,
        stats.missing,
        stats.too_large,
        stats.placements,
        stats.solids,
        stats.triangles,
        stats.textures,
        stats.textures_failed,
        stats.ground,
        stats.ground_skipped,
        stats.bytes as f64 / 1e6,
        stats.picture_millis,
        stats.ground_millis,
        stats.write_millis
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = Options { max_texture: args.max_texture, brightness: args.brightness };
    let opened = Instant::now();
    let install = Install::open(&args.install)?;
    println!("opened the install in {} ms: {} units", opened.elapsed().as_millis(), install.zones().len());
    let numbered = numbered(install.zones());
    let wanted: Vec<(u32, u32)> = numbered
        .iter()
        .filter(|(zone, _)| args.unit.is_none_or(|unit| zone.unit == unit))
        .map(|(zone, section)| (zone.unit, *section))
        .collect();
    anyhow::ensure!(!wanted.is_empty(), "unit {:?} is not in the zone list", args.unit);
    let (mut total_bytes, mut total_ms, mut worst) = (0usize, 0u128, 0u128);
    let mut missing: Vec<u64> = Vec::new();
    for &(unit, section) in &wanted {
        let converted = match convert_unit(&install, unit, section, &options) {
            Ok(converted) => converted,
            Err(error) => {
                println!("unit {unit:5}: failed: {error:#}");
                continue;
            }
        };
        println!("{}", line(unit, &converted.stats));
        for &id in &converted.stats.missing_ids {
            if !missing.contains(&id) {
                missing.push(id);
            }
        }
        total_bytes += converted.bytes.len();
        total_ms += converted.stats.millis;
        worst = worst.max(converted.stats.millis);
        if let (Some(path), 1) = (&args.out, wanted.len()) {
            std::fs::write(path, &converted.bytes).with_context(|| format!("writing {}", path.display()))?;
        }
    }
    if args.find_missing {
        println!("{} models are missing; looking for them in every bundle of the install", missing.len());
        for (id, bundles) in fairhaven::install::bundles_with(&args.install, &missing) {
            println!(
                "   {id:016X}: {}",
                if bundles.is_empty() { "in no bundle".to_owned() } else { bundles.join(", ") }
            );
        }
    }
    println!(
        "{} units, {:.1} MB in all, {total_ms} ms in all, {worst} ms the slowest",
        wanted.len(),
        total_bytes as f64 / 1e6
    );
    Ok(())
}
