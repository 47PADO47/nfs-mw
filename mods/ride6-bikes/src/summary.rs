//! `built.json`: what the manifest needs of a bike's meshes (the hubs, the tyres, the body box), kept beside the bike's
//! `parts.json` when the importer has read the meshes.
//!
//! The meshes and textures of a bike are large and a batch deletes them once the car files are written; the physics, on the
//! other hand, may be converted again and again as the importer learns more. With `built.json` the `remanifest` command
//! rebuilds `cars/<id>.toml` from the physics data and this summary alone, without the meshes.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::build::{Built, Wheel};

/// The file name beside `parts.json`.
pub const FILE: &str = "built.json";

#[derive(Serialize, Deserialize)]
struct WheelSummary {
    hub: [f32; 3],
    radius: f32,
    width: f32,
}

impl From<Wheel> for WheelSummary {
    fn from(wheel: Wheel) -> Self {
        Self { hub: wheel.hub, radius: wheel.radius, width: wheel.width }
    }
}

impl From<WheelSummary> for Wheel {
    fn from(wheel: WheelSummary) -> Self {
        Self { hub: wheel.hub, radius: wheel.radius, width: wheel.width }
    }
}

/// The numbers of [`Built`] the manifest reads.
#[derive(Serialize, Deserialize)]
pub struct Summary {
    /// The vehicle id of the part table.
    vehicle: String,
    half: [f32; 3],
    front: WheelSummary,
    rear: WheelSummary,
    triangles: usize,
}

impl Summary {
    pub fn of(vehicle: &str, built: &Built) -> Self {
        Self {
            vehicle: vehicle.to_owned(),
            half: built.half,
            front: built.front.into(),
            rear: built.rear.into(),
            triangles: built.triangles,
        }
    }

    /// The vehicle id the summary was made for.
    pub fn vehicle(&self) -> &str {
        &self.vehicle
    }

    /// A [`Built`] with these numbers and no files.
    pub fn into_built(self) -> Built {
        Built {
            geometry: Vec::new(),
            textures: Vec::new(),
            bounds: Vec::new(),
            half: self.half,
            front: self.front.into(),
            rear: self.rear.into(),
            triangles: self.triangles,
            front_end: Default::default(),
        }
    }

    pub fn write(&self, dir: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(dir.join(FILE), text).with_context(|| format!("writing {}", dir.join(FILE).display()))
    }

    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join(FILE);
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_reads_back_the_numbers_it_was_made_from() {
        let wheel = |x: f32| Wheel { hub: [x, 0.0, 0.3], radius: 0.3, width: 0.12 };
        let built = Built {
            geometry: vec![1],
            textures: vec![2],
            bounds: vec![3],
            half: [0.4, 0.5, 0.9],
            front: wheel(0.7),
            rear: wheel(-0.7),
            triangles: 77,
            front_end: Default::default(),
        };
        let dir = std::env::temp_dir().join(format!("ride6-summary-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Summary::of("KTM_450-SMR-2022", &built).write(&dir).unwrap();
        let read = Summary::read(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read.vehicle(), "KTM_450-SMR-2022");
        let back = read.into_built();
        assert_eq!((back.half, back.front, back.rear, back.triangles), (built.half, built.front, built.rear, 77));
        assert!(back.geometry.is_empty());
    }
}
