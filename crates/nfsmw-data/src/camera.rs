//! The original game's gameplay camera views: the seven `camerainfo` collections that the cars name in their
//! `ecar` records, in the index order the camera code switches on. Spec: `docs/specs/camera-views.md`.
//!
//! Field names were resolved by matching their `vlt_hash` against the class's field keys (see the spec's "Names"
//! section). The meaning of each field is inferred from its values; the spec lists what is still unconfirmed.

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

/// One view's values: fullscreen and horizontal split-screen. The native POV
/// getter selects the second slot only for horizontal split-screen mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Field of view, degrees.
    pub fov: [f32; 2],
    /// Height of the camera above the car's origin, metres.
    pub height: [f32; 2],
    /// Offset along the car's heading, metres (negative is behind the car; the sign convention is inferred).
    pub lag: [f32; 2],
    /// Pitch, degrees (the look angle; the sign convention is inferred).
    pub angle: [f32; 2],
    /// Stiffness: how firmly the camera holds its place, 0..1 (the use is not confirmed).
    pub stiffness: [f32; 2],
}

impl View {
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

/// The seven views for one car: the car's own variant of each view where the install has one (`<car>_<view>`, the
/// car's name in lower case, for example `bmwm3gtr_hood`), otherwise the generic view.
pub fn load_car(db: &Database, car: &str) -> Option<Views> {
    let mut views = load(db)?.views;
    let car = car.to_ascii_lowercase();
    for (i, (name, _)) in VIEWS.iter().enumerate() {
        let variant = db.collection(CLASS, &format!("{car}_{name}")).and_then(|c| read_view(&c));
        if let Some(view) = variant {
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
    })
}

/// A two-value float array, as the views store each field.
fn pair(value: &Value) -> Option<[f32; 2]> {
    let items = value.as_array()?;
    Some([items.first()?.as_f32()?, items.get(1)?.as_f32()?])
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
    #[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
    fn a_car_uses_its_own_variants_and_falls_back_to_the_generic_views() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let generic = load(&db).expect("generic views");
        let bmw = load_car(&db, "BMWM3GTR").expect("bmw views");
        let own_hood = read_view(&db.collection(CLASS, "bmwm3gtr_hood").unwrap()).unwrap();
        assert_eq!(bmw.views[1], own_hood);
        let unknown = load_car(&db, "NOSUCHCAR").expect("fallback views");
        assert_eq!(unknown, generic);
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
    }
}
