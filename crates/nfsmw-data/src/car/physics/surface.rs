//! Road surface grip: the `simsurface` class (`docs/specs/vehicle-suspension-tires.md` §3.9).
//! Collision hits carry the surface's name hash; the grips come from here.

use std::collections::HashMap;

use blackbox_attrib::{Database, vlt_hash};
use blackbox_vehicle::ground::SurfaceGrip;

use super::fields::Fields;

/// The surface a hash with no collection of its own is looked up as.
const UNKNOWN: &str = "unknown";

/// What a surface sounds like: the `Aud_Skid_Type` (which skid loops) and `Aud_Roadnoise_LOOP` (which road
/// noise loop, 0 = none) fields of `simsurface`. Spec: `docs/specs/engine-sound-effects.md` §7 and §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceAudio {
    pub skid_type: u8,
    pub road_loop: u8,
}

/// The `simsurface` collections by name hash. Every [`SurfaceGrip`] it hands out carries that hash as its `tag`,
/// which [`SurfaceTable::audio`] and the collision sound tables take back.
#[derive(Debug, Clone, Default)]
pub struct SurfaceTable {
    grips: HashMap<u32, SurfaceGrip>,
    audio: HashMap<u32, SurfaceAudio>,
    unknown: SurfaceGrip,
}

impl SurfaceTable {
    pub fn from_database(db: &Database) -> Self {
        let read = |c: Fields<'_>| SurfaceGrip {
            lateral: c.f32("LATERAL_GRIP"),
            drive: c.f32("DRIVE_GRIP"),
            rolling: c.f32("ROLLING_RESISTANCE"),
            tag: c.0.key(),
        };
        let grips = db.collections_of("simsurface").map(|c| (c.key(), read(Fields(c)))).collect();
        let audio = db
            .collections_of("simsurface")
            .map(|c| {
                let byte = |name: &str| c.get_u32(name).unwrap_or(0).min(u32::from(u8::MAX)) as u8;
                (c.key(), SurfaceAudio { skid_type: byte("Aud_Skid_Type"), road_loop: byte("Aud_Roadnoise_LOOP") })
            })
            .collect();
        let unknown = db.collection("simsurface", UNKNOWN).map_or(SurfaceGrip::DEFAULT, |c| read(Fields(c)));
        Self { grips, audio, unknown }
    }

    /// The sounds of a surface by name hash (`unknown` for one without a collection).
    pub fn audio(&self, hash: u32) -> SurfaceAudio {
        self.audio.get(&hash).or_else(|| self.audio.get(&self.unknown.tag)).copied().unwrap_or_default()
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
            grips: HashMap::from([(7, SurfaceGrip { lateral: 0.5, drive: 0.6, rolling: 2.0, tag: 7 })]),
            audio: HashMap::from([(7, SurfaceAudio { skid_type: 1, road_loop: 3 })]),
            unknown: SurfaceGrip { lateral: 0.9, drive: 0.9, rolling: 1.0, tag: 0 },
        };
        assert_eq!(table.grip(7).lateral, 0.5);
        assert_eq!(table.grip(0).lateral, 0.9);
        assert_eq!(table.grip(12345).drive, 0.9);
        assert!(table.knows(7) && !table.knows(0));
        assert_eq!(table.audio(7), SurfaceAudio { skid_type: 1, road_loop: 3 });
        assert_eq!(table.audio(99), SurfaceAudio::default());
    }
}
