//! The original game's gameplay camera views: the seven `camerainfo` collections that the cars name in their
//! `ecar` records, in the index order the camera code switches on. Spec: `docs/specs/camera-views.md`.
//!
//! Native profile references and viewport fields: `docs/specs/original-camera-data.md`.

use blackbox_attrib::{Database, Value, vlt_hash};

/// The attribute class of a camera view.
pub const CLASS: &str = "camerainfo";

/// The seven views, in the index the game's view selection switches on, with the collection name hash
/// (`vlt_hash` of the name) of each view's `camerainfo` collection.
pub const VIEWS: [(&str, u32); 7] = [
    ("bumper", 0x1F4B_1746),
    ("hood", 0x2018_93D0),
    ("close", 0xC6EC_EB8C),
    ("far", 0x18EC_50C6),
    ("superfar", 0x32C8_A5FF),
    ("drift", 0x2907_6EF4),
    ("pursuit", 0xDAA2_52C2),
];

/// The index of the view the game uses by default (the car camera), `far`.
pub const DEFAULT_INDEX: usize = 3;

/// Native model attributes supplying the seven camera profile references.
pub const CAR_FIELDS: [&str; 7] = [
    "CameraInfo_Bumper",
    "CameraInfo_Hood",
    "CameraInfo_Close",
    "CameraInfo_Far",
    "CameraInfo_SuperFar",
    "CameraInfo_Drift",
    "CameraInfo_Pursuit",
];

/// One view's values: fullscreen and horizontal split-screen. The native POV
/// getter selects the second slot only for horizontal split-screen mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Horizontal field of view, degrees; native projection has an independent vertical scale.
    pub fov: [f32; 2],
    /// Height of the camera above its physical anchor, metres.
    pub height: [f32; 2],
    /// Forward offset from the camera anchor, metres (negative is behind).
    pub lag: [f32; 2],
    /// Pitch, degrees (positive looks down).
    pub angle: [f32; 2],
    /// Controller stiffness; not a viewport interpolation fraction.
    pub stiffness: [f32; 2],
    /// Lateral eye offset from the camera anchor, metres.
    pub lat_offset: [f32; 2],
    /// Whether the native target permits the vehicle's up-vector modifier.
    pub tilting: [bool; 2],
    /// Whether the view participates in the ordinary gameplay camera cycle.
    pub selectable: [bool; 2],
}

impl View {
    /// Native horizontal split-screen mode is the only mode using slot one.
    pub fn viewport_index(mode: u32) -> usize {
        usize::from(mode == 3)
    }

    /// Linear interpolation of a pair for callers that explicitly need it.
    /// Native camera POV selection uses a discrete viewport-mode index.
    pub fn at(pair: [f32; 2], speed01: f32) -> f32 {
        pair[0] + (pair[1] - pair[0]) * speed01.clamp(0.0, 1.0)
    }
}

/// The seven views, in `VIEWS` order.
#[derive(Debug, Clone, PartialEq)]
pub struct Views {
    pub views: [View; 7],
}

/// Loads the seven views. Returns `None` if the class or any of the seven collections is missing.
pub fn load(db: &Database) -> Option<Views> {
    let class = vlt_hash(CLASS);
    let views = VIEWS
        .iter()
        .map(|&(_, key)| db.collection_by_key(class, key).and_then(|c| read_view(&c)))
        .collect::<Option<Vec<_>>>()?;
    Some(Views { views: views.try_into().ok()? })
}

/// The seven views selected by the car's actual inherited `ecar` references.
/// Missing cars or unusable references retain the generic view as a host fallback.
pub fn load_car(db: &Database, car: &str) -> Option<Views> {
    let mut views = load(db)?.views;
    let Some(car) = db.collection("ecar", &car.to_ascii_lowercase()) else {
        return Some(Views { views });
    };
    for (i, field) in CAR_FIELDS.iter().enumerate() {
        if let Some(view) = car.follow(field).filter(|c| c.data().class == vlt_hash(CLASS)).and_then(|c| read_view(&c))
        {
            views[i] = view;
        }
    }
    Some(Views { views })
}

fn read_view(c: &blackbox_attrib::CollectionRef<'_>) -> Option<View> {
    Some(View {
        fov: pair(c.get("FOV")?)?,
        height: pair(c.get("HEIGHT")?)?,
        lag: pair(c.get("LAG")?)?,
        angle: pair(c.get("ANGLE")?)?,
        stiffness: pair(c.get("STIFFNESS")?)?,
        lat_offset: pair(c.get("LATOFFSET")?)?,
        tilting: bool_pair(c.get("TILTING")?)?,
        selectable: bool_pair(c.get("SELECTABLE")?)?,
    })
}

/// A two-value float array, as the views store each field.
fn pair(value: &Value) -> Option<[f32; 2]> {
    let items = value.as_array()?;
    Some([items.first()?.as_f32()?, items.get(1)?.as_f32()?])
}

fn bool_pair(value: &Value) -> Option<[bool; 2]> {
    let items = value.as_array()?;
    Some([items.first()?.as_bool()?, items.get(1)?.as_bool()?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_keys_are_the_hashes_of_their_names() {
        for (name, key) in VIEWS {
            assert_eq!(vlt_hash(name), key, "{name}");
        }
    }

    #[test]
    fn the_optional_pair_interpolation_helper_is_linear() {
        assert_eq!(View::at([78.0, 90.0], 0.0), 78.0);
        assert_eq!(View::at([78.0, 90.0], 1.0), 90.0);
        assert_eq!(View::at([78.0, 90.0], 0.5), 84.0);
        assert_eq!(View::at([78.0, 90.0], 2.0), 90.0, "fractions are clamped");
    }

    #[test]
    fn only_horizontal_split_screen_uses_the_second_slot() {
        for mode in 0..=9 {
            assert_eq!(View::viewport_index(mode), usize::from(mode == 3));
        }
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn a_car_uses_actual_camera_references_and_unknown_cars_use_generic_views() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let generic = load(&db).expect("generic views");
        let bmw = load_car(&db, "BMWM3GTR").expect("bmw views");
        let own_hood = read_view(&db.collection("ecar", "bmwm3gtr").unwrap().follow(CAR_FIELDS[1]).unwrap()).unwrap();
        assert_eq!(bmw.views[1], own_hood);
        let unknown = load_car(&db, "NOSUCHCAR").expect("fallback views");
        assert_eq!(unknown, generic);
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn every_installed_ecar_camera_reference_is_used_with_all_viewport_fields() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let mut references = 0;
        for car in db.collections_of("ecar") {
            let name = car.name().expect("ecar collection name");
            let loaded = load_car(&db, name).expect("car camera views");
            for (index, field) in CAR_FIELDS.iter().enumerate() {
                let profile = car.follow(field).expect("installed camera reference resolves");
                let expected = read_view(&profile).expect("all eight paired profile fields");
                assert_eq!(loaded.views[index], expected, "{name}: {field}");
                references += 1;
            }
        }
        assert!(references >= 7, "selected no complete car camera records");
    }

    #[test]
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn base_game_has_the_seven_views() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let views = load(&db).expect("the seven installed views");
        // The default view (`far`) as the install stores it.
        assert_eq!(views.views[DEFAULT_INDEX].fov, [90.0, 90.0]);
        assert_eq!(views.views[DEFAULT_INDEX].height, [1.7, 1.85]);
        assert_eq!(views.views[DEFAULT_INDEX].lat_offset, [0.0, 0.0]);
        assert_eq!(views.views[DEFAULT_INDEX].tilting, [true, true]);
        for (index, view) in views.views.iter().enumerate() {
            assert_eq!(view.selectable, [index <= DEFAULT_INDEX; 2]);
        }
    }
}
