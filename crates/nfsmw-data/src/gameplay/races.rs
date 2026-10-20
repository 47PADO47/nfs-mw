//! The race events: a non-template collection with a `racestart`. Spec: `docs/formats/gameplay-races.md` sections 1-3.

use blackbox_attrib::{CollectionRef, Database, Value};

use super::params::RaceParams;
use super::{collection, instances, key_array, key_field, name_of, placement, text};

/// Where something stands: a position in physics space and the heading in degrees (see [`super::world_heading`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub position: glam::Vec3,
    pub heading_deg: f32,
}

/// The race type the game reads from `EventIconType` (`GRace::Type`, the index in the table below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaceType {
    /// `p2p`: point to point (sprint).
    Sprint = 0,
    /// `circuit`: laps through the same checkpoints.
    Circuit = 1,
    /// `drag`.
    Drag = 2,
    /// `knockout`: a racer is knocked out each lap.
    Knockout = 3,
    /// `tollbooth`.
    Tollbooth = 4,
    /// `speedtrap`: ranked by points from the speed traps.
    SpeedTrap = 5,
    /// `checkpointrace`: occurs in no race of NFS: Most Wanted.
    CheckpointRace = 6,
    /// `cashgrab`: occurs in no race of NFS: Most Wanted.
    CashGrab = 7,
    /// `challenge`: the pursuit challenges.
    Challenge = 8,
    /// `speedtrapjump`: occurs in no race of NFS: Most Wanted.
    SpeedTrapJump = 9,
    /// `milestonejump`: occurs in no race of NFS: Most Wanted.
    MilestoneJump = 10,
}

impl RaceType {
    /// The type of an `EventIconType` string. `None` for an empty or unknown icon.
    pub fn from_icon(icon: &str) -> Option<Self> {
        Some(match icon {
            "p2p" => Self::Sprint,
            "circuit" => Self::Circuit,
            "drag" => Self::Drag,
            "knockout" => Self::Knockout,
            "tollbooth" => Self::Tollbooth,
            "speedtrap" => Self::SpeedTrap,
            "checkpointrace" => Self::CheckpointRace,
            "cashgrab" => Self::CashGrab,
            "challenge" => Self::Challenge,
            "speedtrapjump" => Self::SpeedTrapJump,
            "milestonejump" => Self::MilestoneJump,
            _ => return None,
        })
    }
}

/// The field of a race that holds its event id, `19.8.18` (name unknown).
const EVENT_FIELD: u32 = 0xA784_03EC;

/// One race or challenge.
#[derive(Debug, Clone, PartialEq)]
pub struct Race {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `race_bin_10/10_2_1_sprint`.
    pub name: String,
    /// `EventIconType` as written, for example `p2p`. Empty for the story shells.
    pub event_icon: String,
    /// The type of `event_icon`; `None` when the icon is empty or not in the game's table.
    pub race_type: Option<RaceType>,
    /// The event id the race stores (`19.8.18`), field `0xA78403EC` (name not known). The pursuit challenges and the
    /// side bins carry it; their data names do not hold it.
    pub event: Option<String>,
    /// `Region` (`college`, `coastal`, `city`).
    pub region: Option<String>,
    /// `NumLaps`.
    pub laps: i32,
    /// `racestart`: the front of the start grid.
    pub start: Option<Placement>,
    /// `racefinish`: the key of the finish line, a [`super::BoxTrigger`] (see [`super::GameplayData::trigger`]).
    /// `None` in a few story shells.
    pub finish: Option<u32>,
    /// `Checkpoint[]`: the keys of the checkpoints in running order, each a [`super::BoxTrigger`].
    pub checkpoints: Vec<u32>,
    /// `Barriers[]`: the track-path barrier names this race enables, as written (a leading `*` is kept, and
    /// entries are not trimmed). Spec: `docs/specs/career-regions.md` section 3.4. The career crate parses them.
    pub barriers: Vec<String>,
    /// The start, clock, opponent and rank rules of the run (`docs/specs/race-mode.md`).
    pub params: RaceParams,
}

/// The races of a database, in database order. Empty when the database has no `gameplay` class.
pub(crate) fn races(db: &Database) -> Vec<Race> {
    instances(db).filter_map(|c| race(db, c)).collect()
}

fn race(db: &Database, c: CollectionRef<'_>) -> Option<Race> {
    // Only races set `racestart`; templates and everything else are skipped by `instances`.
    let start_key = key_field(c, "racestart")?;
    let event_icon = text(c, "EventIconType").unwrap_or_default();
    Some(Race {
        key: c.key(),
        name: name_of(c),
        race_type: RaceType::from_icon(&event_icon),
        event_icon,
        event: c.get_by_key(EVENT_FIELD).and_then(Value::as_str).filter(|id| !id.is_empty()).map(str::to_owned),
        region: text(c, "Region"),
        laps: c.get_i32("NumLaps").unwrap_or(0),
        start: collection(db, start_key).map(placement),
        finish: key_field(c, "racefinish"),
        checkpoints: key_array(c, "Checkpoint"),
        barriers: text_items(c.get("Barriers")),
        params: RaceParams::read(c),
    })
}

/// The text items of a text array field, in order, as written. A scalar text is a one-item list; null or
/// non-text items are skipped; a missing field is an empty list.
fn text_items(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        Some(value) => value.as_str().map(str::to_owned).into_iter().collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_map_to_the_game_race_types() {
        assert_eq!(RaceType::from_icon("p2p"), Some(RaceType::Sprint));
        assert_eq!(RaceType::from_icon("tollbooth"), Some(RaceType::Tollbooth));
        assert_eq!(RaceType::from_icon("milestonejump"), Some(RaceType::MilestoneJump));
        assert_eq!(RaceType::from_icon(""), None);
        assert_eq!(RaceType::from_icon("sprint"), None);
    }

    #[test]
    fn race_type_indices_follow_the_game_table() {
        let icons = [
            "p2p",
            "circuit",
            "drag",
            "knockout",
            "tollbooth",
            "speedtrap",
            "checkpointrace",
            "cashgrab",
            "challenge",
            "speedtrapjump",
            "milestonejump",
        ];
        for (i, icon) in icons.iter().enumerate() {
            let t = RaceType::from_icon(icon).unwrap();
            assert_eq!(t as u8 as usize, i, "{icon}");
        }
    }

    #[test]
    fn barrier_items_keep_the_star_and_skip_non_text() {
        let list = Value::Array(vec![
            Value::Text(Some("*BARRIER_SPLINE_3".to_owned())),
            Value::Text(None),
            Value::Text(Some("SCENERY_GROUP_DOOR".to_owned())),
            Value::Int32(7),
        ]);
        assert_eq!(text_items(Some(&list)), vec!["*BARRIER_SPLINE_3", "SCENERY_GROUP_DOOR"]);
    }

    #[test]
    fn barrier_items_of_a_scalar_or_missing_field() {
        let scalar = Value::Text(Some("BARRIER_LOCKED_12".to_owned()));
        assert_eq!(text_items(Some(&scalar)), vec!["BARRIER_LOCKED_12"]);
        assert!(text_items(None).is_empty());
        assert!(text_items(Some(&Value::Array(Vec::new()))).is_empty());
    }
}
