//! `inventory`: what the car files of imported bikes hold, solid by solid, as JSON for `tools/ride6-exporter/completeness.py`.
//!
//! The completeness report compares what RIDE 6 draws for a stock bike (read from the exporter's meshes) with what the
//! importer wrote. The importer's own numbers could repeat a mistake of the importer, so this reads the files back: every
//! `files/CARS/<ID>/GEOMETRY.BIN` of a mod folder, each solid with its vertices, triangles and shading groups.
//!
//! ```text
//! inventory <mod folder> [--only <text>]     one JSON line per car: {"car": .., "solids": [{"name": .., "vertices": .., ..}]}
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

#[derive(Serialize)]
struct Group {
    vertices: u32,
    triangles: u32,
    effect: u32,
    light_material: u8,
    texture_slot: u8,
}

#[derive(Serialize)]
struct SolidInfo {
    name: String,
    vertices: usize,
    triangles: usize,
    groups: Vec<Group>,
    min: [f32; 3],
    max: [f32; 3],
}

#[derive(Serialize)]
struct Car {
    car: String,
    solids: Vec<SolidInfo>,
}

#[derive(Parser)]
#[command(about = "Print the solids of the imported bikes' GEOMETRY.BIN files as JSON lines")]
struct Args {
    /// The mod folder (`files/CARS/<ID>/GEOMETRY.BIN` of each car).
    mod_dir: PathBuf,
    /// Only the cars whose folder name contains this text (case-insensitive).
    #[arg(long)]
    only: Option<String>,
}

/// The solids of one car file.
fn read_car(car: &str, geometry: &Path) -> Result<Car> {
    let bytes = std::fs::read(geometry).with_context(|| format!("reading {}", geometry.display()))?;
    let solids = blackbox_solid::read_solids(&bytes).with_context(|| format!("decoding {}", geometry.display()))?;
    let solids = solids
        .into_iter()
        .map(|solid| SolidInfo {
            vertices: solid.vertices.len(),
            triangles: solid.groups.iter().map(|g| g.num_triangles as usize).sum(),
            groups: solid
                .groups
                .iter()
                .map(|g| Group {
                    vertices: g.num_vertices,
                    triangles: g.num_triangles,
                    effect: g.effect_id,
                    light_material: g.light_material,
                    texture_slot: g.texture_slots[0],
                })
                .collect(),
            min: solid.bounds_min,
            max: solid.bounds_max,
            name: solid.name,
        })
        .collect();
    Ok(Car { car: car.to_owned(), solids })
}

/// `inventory`: print one JSON line per car of the mod folder.
pub fn run(args: &[String]) -> Result<()> {
    let args = Args::try_parse_from(std::iter::once("inventory".to_owned()).chain(args.iter().cloned()))?;
    let cars = args.mod_dir.join("files").join("CARS");
    let mut names: Vec<String> = std::fs::read_dir(&cars)
        .with_context(|| format!("listing {}", cars.display()))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    let only = args.only.as_deref().map(str::to_ascii_lowercase);
    for name in names {
        if only.as_deref().is_some_and(|text| !name.to_ascii_lowercase().contains(text)) {
            continue;
        }
        let geometry = cars.join(&name).join("GEOMETRY.BIN");
        if !geometry.is_file() {
            continue;
        }
        match read_car(&name, &geometry) {
            Ok(car) => println!("{}", serde_json::to_string(&car)?),
            Err(error) => eprintln!("{name}: {error:#}"),
        }
    }
    Ok(())
}
