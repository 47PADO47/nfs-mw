//! The `gameplay` class of `GLOBAL/gameplay.bin`: the race events, the box triggers they use (start, finish,
//! checkpoints) and the activity markers of the auto shops and safe houses. Layouts and evidence:
//! `docs/formats/gameplay-races.md`.
//!
//! Positions are stored in map coordinates (x, y, z with z up). They are converted to physics space with
//! [`to_world`], `world = (-y, z, x)`. Headings are kept in degrees as stored; [`world_heading`] gives the unit
//! vector.

mod bins;
mod final_race;
mod markers;
mod milestones;
mod params;
mod races;
mod rewards;
mod story;
mod triggers;

pub use bins::{Bin, bins};
pub use final_race::{EventMarker, FinalRaceMarkers};
pub use markers::{ActivityKind, ActivityMarker};
pub use milestones::{
    Milestone, MilestoneType, RivalRequirements, SpeedTrap, milestone_types, milestones, rival_requirements,
    speed_traps,
};
pub use params::{Character, RaceParams};
pub use races::{Placement, Race, RaceType};
pub use rewards::RewardRecord;
pub use story::RaceStory;
pub use triggers::{BoxTrigger, SphereTrigger};

use anyhow::{Context, Result};
use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};
use game_install::GameDir;
use glam::Vec3;

use crate::read_unwrapped;
use crate::shared;

/// The file with the gameplay collections (the `gameplay` class, all 6,293 of them in NFS: Most Wanted).
pub const GAMEPLAY_FILE: &str = "GLOBAL/gameplay.bin";
/// The start point collection of the career is `career_start_` and the region's name (the install has `l2ra`).
const CAREER_START_PREFIX: &str = "career_start_";
/// The class holding every collection read here.
const CLASS: &str = "gameplay";

/// Everything read from the `gameplay` class that this module covers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameplayData {
    /// Every race and challenge (non-template collections with a `racestart`), in database order.
    pub races: Vec<Race>,
    /// Every box trigger (collections that inherit `boxtrigger`): race finish lines and checkpoints, among others.
    pub triggers: Vec<BoxTrigger>,
    /// Every sphere trigger (collections that inherit `radiustrigger`): the speed traps of the races.
    pub spheres: Vec<SphereTrigger>,
    /// The auto shop and safe house zones, in database order.
    pub markers: Vec<ActivityMarker>,
    /// Every opponent driver (`character`, `boss`, `character_smart`), in database order.
    pub characters: Vec<Character>,
    /// The career Blacklist bins, sorted by bin number.
    pub bins: Vec<Bin>,
    /// The 55 pursuit milestones, sorted by bin then name.
    pub milestones: Vec<Milestone>,
    /// The 39 speed traps, sorted by bin then name.
    pub speed_traps: Vec<SpeedTrap>,
    /// The 30 milestone statistic types (from `attributes.bin`, which [`load`] puts in the database).
    pub milestone_types: Vec<MilestoneType>,
    /// The final race's markers (`race_bin_01/epic_pursuit`); `None` when the database has no such race.
    pub final_race: Option<FinalRaceMarkers>,
    /// The career's start point, the collection `career_start_<region>`: where a world starts when no safe house or zone
    /// was visited (`docs/specs/career-flow.md` section 6).
    pub career_start: Option<Placement>,
}

impl GameplayData {
    /// The box trigger with this collection key (a race's `finish` or one of its `checkpoints`).
    pub fn trigger(&self, key: u32) -> Option<&BoxTrigger> {
        self.triggers.iter().find(|t| t.key == key)
    }

    /// The sphere trigger with this collection key (an entry of a race's `SpeedTrapList`).
    pub fn sphere(&self, key: u32) -> Option<&SphereTrigger> {
        self.spheres.iter().find(|t| t.key == key)
    }

    /// The opponent character with this collection key (an entry of a race's `Opponents`).
    pub fn character(&self, key: u32) -> Option<&Character> {
        self.characters.iter().find(|c| c.key == key)
    }

    /// The race with this full collection name (for example `race_bin_10/10_2_1_sprint`).
    pub fn race_named(&self, name: &str) -> Option<&Race> {
        self.races.iter().find(|r| r.name == name)
    }

    /// The `respawn` marker of the activity zone with this full name (for example
    /// `menu_gate_safehouse/zone_safehouse_1`): where the car goes back to after a bust there.
    pub fn zone_respawn(&self, zone: &str) -> Option<Placement> {
        self.markers.iter().find(|m| m.name == zone)?.respawn
    }

    /// Read a database that holds `attributes.bin` and `gameplay.bin` (see [`load`]).
    pub fn from_database(db: &Database) -> Self {
        Self {
            races: races::races(db),
            triggers: triggers::triggers(db),
            spheres: triggers::spheres(db),
            markers: markers::markers(db),
            characters: params::characters(db),
            bins: bins::bins(db),
            milestones: milestones::milestones(db),
            speed_traps: milestones::speed_traps(db),
            milestone_types: milestones::milestone_types(db),
            final_race: final_race::final_race(db),
            career_start: instances(db).find(|c| name_of(*c).starts_with(CAREER_START_PREFIX)).map(placement),
        }
    }
}

/// Read the install's gameplay data: class definitions from `GLOBAL/ATTRIBUTES.BIN`, collections from
/// `GLOBAL/gameplay.bin`.
pub fn load(dir: &GameDir) -> Result<GameplayData> {
    // The session's class definitions are shared; the collections go into this call's own copy.
    let attributes = shared::attributes(dir)?;
    let mut db = Database::clone(&attributes);
    let gameplay = read_unwrapped(dir, GAMEPLAY_FILE)?;
    db.load(&gameplay).with_context(|| format!("parsing {GAMEPLAY_FILE}"))?;
    Ok(GameplayData::from_database(&db))
}

/// Map coordinates to physics space: `world = (-y, z, x)`.
pub fn to_world(map: [f32; 3]) -> Vec3 {
    Vec3::new(-map[1], map[2], map[0])
}

/// The unit heading of a `Rotation` (degrees) in physics space: `(-sin R, 0, cos R)`.
pub fn world_heading(rotation_deg: f32) -> Vec3 {
    let r = rotation_deg.to_radians();
    Vec3::new(-r.sin(), 0.0, r.cos())
}

/// The gameplay collection with this key, if it is loaded.
pub(crate) fn collection(db: &Database, key: u32) -> Option<CollectionRef<'_>> {
    db.collection_by_key(vlt_hash(CLASS), key)
}

/// Every collection of the `gameplay` class that is not a template.
pub(crate) fn instances(db: &Database) -> impl Iterator<Item = CollectionRef<'_>> {
    db.collections_of(CLASS).filter(|c| !is_template(*c))
}

/// `Template` is inherited, so a template is one whose lineage sets it (every instance sets it to False).
pub(crate) fn is_template(c: CollectionRef<'_>) -> bool {
    c.get_bool("Template").unwrap_or(false)
}

/// Whether `c` or one of its ancestors is the collection `ancestor` (matched by name hash).
pub(crate) fn inherits(c: CollectionRef<'_>, ancestor: &str) -> bool {
    let key = vlt_hash(ancestor);
    c.lineage().any(|a| a.key() == key)
}

/// The collection's name: its `CollectionName` text, or the name the database knows it by.
pub(crate) fn name_of(c: CollectionRef<'_>) -> String {
    text(c, "CollectionName").or_else(|| c.name().map(str::to_owned)).unwrap_or_default()
}

/// A text field, trimmed; `None` when unset or empty.
pub(crate) fn text(c: CollectionRef<'_>, field: &str) -> Option<String> {
    c.get_str(field).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// The position and heading of a marker-like collection (`Position` in map coordinates, `Rotation` in degrees).
pub(crate) fn placement(c: CollectionRef<'_>) -> Placement {
    Placement {
        position: to_world(c.get_vector3("Position").unwrap_or_default()),
        heading_deg: c.get_f32("Rotation").unwrap_or(0.0),
    }
}

/// A `GCollectionKey`: the 4-byte hash of another collection's name (0 for none). The database keeps these as raw
/// bytes, so they are read here.
pub(crate) fn key_of(value: &Value) -> Option<u32> {
    match value {
        Value::Key(k) => Some(*k),
        Value::Raw { bytes, .. } => <[u8; 4]>::try_from(bytes.as_slice()).ok().map(u32::from_le_bytes),
        _ => None,
    }
}

/// A single `GCollectionKey` field; `None` when unset or 0.
pub(crate) fn key_field(c: CollectionRef<'_>, field: &str) -> Option<u32> {
    c.get(field).and_then(key_of).filter(|k| *k != 0)
}

/// An array of `GCollectionKey`s, in order, without the empty entries.
pub(crate) fn key_array(c: CollectionRef<'_>, field: &str) -> Vec<u32> {
    key_array_at(c, vlt_hash(field))
}

/// Like [`key_array`], for a field known only by its key.
pub(crate) fn key_array_at(c: CollectionRef<'_>, field_key: u32) -> Vec<u32> {
    match c.get_by_key(field_key) {
        Some(Value::Array(items)) => items.iter().filter_map(key_of).filter(|k| *k != 0).collect(),
        Some(value) => key_of(value).filter(|k| *k != 0).into_iter().collect(),
        None => Vec::new(),
    }
}

/// The key of the child `child` of `c`: the collection named `<c's name>/<child>`, as the race children are named
/// (for example `race_bin_16/16_1_1_circuit/outro_nis_marker`).
pub(crate) fn child_key(c: CollectionRef<'_>, child: &str) -> u32 {
    vlt_hash(&format!("{}/{child}", name_of(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_coordinates_become_physics_space() {
        // The sprint's start grid: map (1511.824, 4404.865, 209.6045) -> world (-4404.865, 209.6045, 1511.824).
        let w = to_world([1511.824, 4404.865, 209.6045]);
        assert_eq!(w, Vec3::new(-4404.865, 209.6045, 1511.824));
    }

    #[test]
    fn rotation_gives_the_heading_in_physics_space() {
        let h0 = world_heading(0.0);
        assert!((h0 - Vec3::Z).length() < 1e-6, "0 degrees points along +z: {h0}");
        let h90 = world_heading(90.0);
        assert!((h90 - Vec3::NEG_X).length() < 1e-6, "90 degrees points along -x: {h90}");
        assert!((world_heading(-37.0).length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn gcollection_keys_are_little_endian_hashes() {
        let raw = Value::Raw { type_key: 0x6CD4_B70A, bytes: vec![21, 81, 19, 151] };
        assert_eq!(key_of(&raw), Some(0x9713_5115));
        assert_eq!(key_of(&Value::Raw { type_key: 0, bytes: vec![1, 2, 3] }), None);
        assert_eq!(key_of(&Value::Key(7)), Some(7));
        assert_eq!(key_of(&Value::Float(1.0)), None);
    }
}
