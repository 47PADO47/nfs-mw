//! The race events: a non-template collection with a `racestart`. Spec: `docs/formats/gameplay-races.md` sections 1-3.

use blackbox_attrib::{CollectionRef, Database};

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
        region: text(c, "Region"),
        laps: c.get_i32("NumLaps").unwrap_or(0),
        start: collection(db, start_key).map(placement),
        finish: key_field(c, "racefinish"),
        checkpoints: key_array(c, "Checkpoint"),
    })
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
}
