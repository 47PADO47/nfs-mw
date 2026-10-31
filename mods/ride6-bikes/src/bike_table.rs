//! The `[bike]` table of the manifest: finds the bike's resolved physics file, reads it and converts it with what the meshes
//! say about the wheels and the body ([`crate::physics`]).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::build::{Built, Wheel};
use crate::physics::vehicle::Vehicle;
use crate::physics::{self, Geometry, TyreGeometry};

/// The text of the `[bike]` table and its sub-tables, and what the conversion wants the user to know.
#[derive(Debug, Clone, PartialEq)]
pub struct BikeTable {
    pub text: String,
    pub notes: Vec<String>,
}

fn tyre(wheel: &Wheel) -> TyreGeometry {
    TyreGeometry { hub: wheel.hub.map(f64::from), radius_m: f64::from(wheel.radius), width_m: f64::from(wheel.width) }
}

/// What the converted meshes say: the hubs, the tyres' size and the body box.
pub fn geometry(built: &Built) -> Geometry {
    Geometry { front: tyre(&built.front), rear: tyre(&built.rear), half_m: built.half.map(f64::from) }
}

/// A vehicle id and a file name compare equal when their letters and digits do: `KTM_450-SMR-2022` and `KTM_450SMR_2022`.
fn squash(id: &str) -> String {
    id.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// The resolved file of a vehicle. `physics` is the file itself, or the dump folder (`physdump`, or its `vehicles` folder)
/// in which the file is found by the vehicle id of the part table.
pub fn locate(physics: &Path, vehicle_id: &str) -> Result<PathBuf> {
    if physics.is_file() {
        return Ok(physics.to_path_buf());
    }
    let root = match physics.join("vehicles").is_dir() {
        true => physics.join("vehicles"),
        false => physics.to_path_buf(),
    };
    let wanted = squash(vehicle_id);
    let brands = fs::read_dir(&root).with_context(|| format!("reading {}", root.display()))?;
    for brand in brands.flatten().filter(|entry| entry.path().is_dir()) {
        let Ok(files) = fs::read_dir(brand.path()) else { continue };
        for file in files.flatten() {
            let path = file.path();
            let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
            if path.extension().is_some_and(|ext| ext == "json") && squash(stem) == wanted {
                return Ok(path);
            }
        }
    }
    bail!("no resolved physics file for `{vehicle_id}` under {}", root.display())
}

/// Convert the vehicle's physics. `tyre_tables` is the `Tire` folder of the dump, for the tyre assets the vehicle file does
/// not link.
pub fn read(physics: &Path, tyre_tables: Option<&Path>, vehicle_id: &str, built: &Built) -> Result<BikeTable> {
    let file = locate(physics, vehicle_id)?;
    let vehicle = Vehicle::read(&file, tyre_tables)?;
    render(&vehicle, &geometry(built)).with_context(|| format!("converting the physics of {}", file.display()))
}

pub fn render(vehicle: &Vehicle, geometry: &Geometry) -> Result<BikeTable> {
    let (text, notes) = physics::bike_table(vehicle, geometry)?;
    Ok(BikeTable { text, notes })
}

#[cfg(test)]
#[path = "bike_table_tests.rs"]
mod tests;
