//! Road surface grip: the `simsurface` class (`docs/specs/vehicle-suspension-tires.md` §3.9).
//! Collision hits carry the surface's name hash; the grips come from here.

use std::collections::HashMap;

use blackbox_attrib::{Database, vlt_hash};
use blackbox_vehicle::ground::SurfaceGrip;

use super::fields::Fields;

/// The surface a hash with no collection of its own is looked up as.
const UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, Default)]
pub struct SurfaceTable {
    grips: HashMap<u32, SurfaceGrip>,
    unknown: SurfaceGrip,
}

impl SurfaceTable {
    pub fn from_database(db: &Database) -> Self {
        let read = |c: Fields<'_>| SurfaceGrip {
            lateral: c.f32("LATERAL_GRIP"),
            drive: c.f32("DRIVE_GRIP"),
            rolling: c.f32("ROLLING_RESISTANCE"),
        };
        let grips = db.collections_of("simsurface").map(|c| (c.key(), read(Fields(c)))).collect();
        let unknown = db.collection("simsurface", UNKNOWN).map_or(SurfaceGrip::DEFAULT, |c| read(Fields(c)));
        Self { grips, unknown }
    }

    /// The grips of a surface name hash (`0` or one without a collection is `unknown`).
    pub fn grip(&self, hash: u32) -> SurfaceGrip {
        self.grips.get(&hash).copied().unwrap_or(self.unknown)
    }

    /// Whether the hash names a surface this table knows.
    pub fn knows(&self, hash: u32) -> bool {
        self.grips.contains_key(&hash)
    }

    pub fn len(&self) -> usize {
        self.grips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grips.is_empty()
    }

    /// The hash of a surface name, as collision hits carry it.
    pub fn hash_of(name: &str) -> u32 {
        vlt_hash(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_hashes_fall_back_to_the_unknown_surface() {
        let table = SurfaceTable {
            grips: HashMap::from([(7, SurfaceGrip { lateral: 0.5, drive: 0.6, rolling: 2.0 })]),
            unknown: SurfaceGrip { lateral: 0.9, drive: 0.9, rolling: 1.0 },
        };
        assert_eq!(table.grip(7).lateral, 0.5);
        assert_eq!(table.grip(0).lateral, 0.9);
        assert_eq!(table.grip(12345).drive, 0.9);
        assert!(table.knows(7) && !table.knows(0));
    }
}
