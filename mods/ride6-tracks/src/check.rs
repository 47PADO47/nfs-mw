//! A check of a converted world's ground against the lap: a ray is cast down onto the collision at every sample of the track
//! spline, and the surface and the height of the first hit are counted. The spline is where the track's authors put the racing
//! surface, so it should be asphalt (or kerb) all the way round, at the height of the spline.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use blackbox_collision::{CollisionWorld, RayOptions, read_collision_packs};

use crate::collision::{surface_hash, to_physics};
use crate::manifest::world_id;
use crate::pack::Pack;

/// The surfaces a track has, to name a hit's surface hash.
const SURFACES: [&str; 8] = ["asphalt", "concrete", "grass", "dirt", "sand", "gravel", "stone", "metal"];

/// What the spline's samples met.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub samples: usize,
    /// Samples with nothing under them.
    pub missed: usize,
    /// How many samples hit each surface.
    pub surfaces: BTreeMap<String, usize>,
    /// The largest distance of a hit from the spline's height, metres.
    pub worst_height: f32,
    /// Samples whose hit is more than a metre from the spline's height.
    pub off_height: usize,
}

/// The collision of the world `pack` was converted to, under `mod_dir`.
pub fn load_world(mod_dir: &Path, pack: &Pack) -> Result<CollisionWorld> {
    let folder = mod_dir.join("files").join("WORLDS").join(world_id(&pack.track).to_ascii_uppercase());
    let mut world = CollisionWorld::new(None);
    let mut files: Vec<_> = std::fs::read_dir(&folder)
        .with_context(|| format!("reading {}", folder.display()))?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    files.sort();
    for path in files {
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        for collision in
            read_collision_packs(&bytes).with_context(|| format!("reading the collision of {}", path.display()))?
        {
            world.insert_ungridded(collision);
        }
    }
    Ok(world)
}

/// Casts a ray down at each sample of the spline of `pack`.
pub fn along_the_lap(world: &CollisionWorld, pack: &Pack) -> Report {
    let mut report = Report::default();
    let names: Vec<(u32, &str)> = SURFACES.iter().map(|name| (surface_hash(name), *name)).collect();
    for point in &pack.lap.spline.points {
        report.samples += 1;
        let [px, py, pz] = to_physics(*point);
        let Some(hit) = world.ray_cast(
            [px, py + 5.0, pz],
            [px, py - 5.0, pz],
            &RayOptions { barriers: false, ..RayOptions::default() },
        ) else {
            report.missed += 1;
            continue;
        };
        let name = names.iter().find(|(hash, _)| *hash == hit.surface_hash).map_or("unknown", |(_, name)| name);
        *report.surfaces.entry(name.to_owned()).or_default() += 1;
        let error = (hit.point[1] - py).abs();
        report.worst_height = report.worst_height.max(error);
        report.off_height += usize::from(error > 1.0);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::import;
    use crate::pack::{self, Group};
    use std::sync::atomic::AtomicBool;

    /// A pack of one asphalt square of 20 m at height 1 and a lap of three samples, the last one off the square.
    fn write_pack(dir: &Path) {
        std::fs::create_dir_all(dir.join("tex")).unwrap();
        let json = r#"{"format": 1, "track": "Flat", "triangles": 2,
            "materials": [{"name": "t", "texture": null, "color": [60, 60, 60], "surface": "asphalt"}],
            "lap": {"grid": [], "spline": {"closed": true, "length_m": 100.0, "points": [[5.0, 5.0, 1.0], [15.0, 10.0, 1.0], [100.0, 100.0, 1.0]]}}}"#;
        std::fs::write(dir.join("pack.json"), json).unwrap();
        let group = Group {
            material: 0,
            collide: true,
            model: 0,
            positions: vec![[0.0, 0.0, 1.0], [20.0, 0.0, 1.0], [20.0, 20.0, 1.0], [0.0, 20.0, 1.0]],
            uvs: vec![[0.0, 0.0]; 4],
            normals: vec![[0, 0, 127]; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
        };
        std::fs::write(dir.join("ground.r6m"), pack::write_groups(&[group])).unwrap();
    }

    #[test]
    fn the_lap_over_a_flat_track_is_all_the_surface_under_it() {
        let out = std::env::temp_dir().join(format!("ride6-tracks-check-{}", std::process::id()));
        let packs = std::env::temp_dir().join(format!("ride6-tracks-check-pack-{}", std::process::id()));
        write_pack(&packs);
        import(&packs, &out, 64, &AtomicBool::new(false)).unwrap();
        let pack = pack::read(&packs).unwrap();
        let world = load_world(&out, &pack).unwrap();
        let report = along_the_lap(&world, &pack);
        assert_eq!(report.samples, 3);
        assert_eq!(report.missed, 1, "the third sample is off the ground");
        assert_eq!(report.surfaces.get("asphalt"), Some(&2));
        assert!(report.worst_height < 0.05, "{}", report.worst_height);
        let _ = std::fs::remove_dir_all(&out);
        let _ = std::fs::remove_dir_all(&packs);
    }
}
