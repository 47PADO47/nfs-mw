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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceAudio {
    pub skid_type: u8,
    pub road_loop: u8,
    pub transition_on: i32,
    pub transition_off: i32,
    pub secondary: bool,
}

impl Default for SurfaceAudio {
    fn default() -> Self {
        Self { skid_type: 0, road_loop: 0, transition_on: -1, transition_off: -1, secondary: false }
    }
}

/// The `simsurface` collections by name hash. Every [`SurfaceGrip`] it hands out carries that hash as its `tag`,
/// which [`SurfaceTable::audio`] and the collision sound tables take back.
#[derive(Debug, Clone, Default)]
pub struct SurfaceTable {
    grips: HashMap<u32, SurfaceGrip>,
    audio: HashMap<u32, SurfaceAudio>,
    body_friction: HashMap<u32, f32>,
    world_friction: HashMap<u32, f32>,
    camera_noise: HashMap<u32, [f32; 2]>,
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
                (
                    c.key(),
                    SurfaceAudio {
                        skid_type: byte("Aud_Skid_Type"),
                        road_loop: byte("Aud_Roadnoise_LOOP"),
                        transition_on: c.get_u32("Aud_RoadNoise_TransON").map_or(-1, |value| value as i32),
                        transition_off: c.get_u32("Aud_RoadNoise_TransOFF").map_or(-1, |value| value as i32),
                        secondary: [vlt_hash("dirt"), vlt_hash("gravel")].contains(&c.key()),
                    },
                )
            })
            .collect();
        let unknown = db.collection("simsurface", UNKNOWN).map_or(SurfaceGrip::DEFAULT, |c| read(Fields(c)));
        let body_friction =
            db.collections_of("simsurface").map(|c| (c.key(), c.get_f32("GROUND_FRICTION").unwrap_or(1.0))).collect();
        let camera_noise = db.collections_of("simsurface").map(|c| (c.key(), camera_pair(c))).collect();
        let world_friction =
            db.collections_of("simsurface").map(|c| (c.key(), c.get_f32("WORLD_FRICTION").unwrap_or(1.0))).collect();
        Self { grips, audio, body_friction, world_friction, camera_noise, unknown }
    }

    pub fn body_friction(&self, hash: u32) -> f32 {
        self.body_friction.get(&hash).or_else(|| self.body_friction.get(&self.unknown.tag)).copied().unwrap_or(1.0)
    }

    /// Original wall/barrier friction, distinct from ground contact friction.
    pub fn world_friction(&self, hash: u32) -> f32 {
        self.world_friction.get(&hash).or_else(|| self.world_friction.get(&self.unknown.tag)).copied().unwrap_or(1.0)
    }

    /// Native terrain-camera values for a contact surface, or the installed unknown surface.
    pub fn camera_noise(&self, hash: u32) -> [f32; 2] {
        self.camera_noise.get(&hash).or_else(|| self.camera_noise.get(&self.unknown.tag)).copied().unwrap_or([0.0; 2])
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

fn camera_pair(surface: blackbox_attrib::CollectionRef<'_>) -> [f32; 2] {
    std::array::from_fn(|index| {
        surface.get_at("CAMERA_NOISE", index).and_then(blackbox_attrib::Value::as_f32).unwrap_or(0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_hashes_fall_back_to_the_unknown_surface() {
        let table = SurfaceTable {
            grips: HashMap::from([(7, SurfaceGrip { lateral: 0.5, drive: 0.6, rolling: 2.0, tag: 7 })]),
            audio: HashMap::from([(7, SurfaceAudio { skid_type: 1, road_loop: 3, ..Default::default() })]),
            body_friction: HashMap::from([(7, 0.3)]),
            world_friction: HashMap::from([(7, 0.8), (0, 0.4)]),
            camera_noise: HashMap::from([(7, [0.6, 0.1]), (0, [0.2, 0.3])]),
            unknown: SurfaceGrip { lateral: 0.9, drive: 0.9, rolling: 1.0, tag: 0 },
        };
        assert_eq!(table.grip(7).lateral, 0.5);
        assert_eq!(table.grip(0).lateral, 0.9);
        assert_eq!(table.grip(12345).drive, 0.9);
        assert!(table.knows(7) && !table.knows(0));
        assert_eq!(table.audio(7), SurfaceAudio { skid_type: 1, road_loop: 3, ..Default::default() });
        assert_eq!(table.audio(99), SurfaceAudio::default());
        assert_eq!(table.world_friction(7), 0.8);
        assert_eq!(table.world_friction(99), 0.4);
        assert_eq!(table.camera_noise(7), [0.6, 0.1]);
        assert_eq!(table.camera_noise(99), [0.2, 0.3]);
        assert_eq!(SurfaceTable::default().camera_noise(99), [0.0; 2]);
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn installed_camera_noise_preserves_each_surface_pair() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = crate::shared::attributes(&dir).unwrap();
        let table = SurfaceTable::from_database(&db);
        let mut count = 0;
        for surface in db.collections_of("simsurface") {
            let expected =
                std::array::from_fn(|index| surface.get_at("CAMERA_NOISE", index).unwrap().as_f32().unwrap());
            assert_eq!(table.camera_noise(surface.key()), expected);
            count += 1;
        }
        assert!(count > 0, "selected no surface camera records");
    }
}
