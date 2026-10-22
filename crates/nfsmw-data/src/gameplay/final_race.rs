//! The markers of the final race, `race_bin_01/epic_pursuit`: the win and lose triggers, the barrier area, the win
//! cutscene marker, the front of the start grid and the six tagged car markers. Spec: `docs/specs/story-flow-2.md`
//! section 5.2 and F-32; `docs/formats/gameplay-races.md` section 6. The children are named by their full names under
//! the race, so they are read by name.

use blackbox_attrib::{CollectionRef, Database, vlt_hash};

use super::triggers::{BoxTrigger, trigger};
use super::{Placement, collection, placement};

/// The full name of the final race's collection.
pub const FINAL_RACE: &str = "race_bin_01/epic_pursuit";

/// A child marker of the final race that is not a box trigger: where it stands, and its size when it stores one.
#[derive(Debug, Clone, PartialEq)]
pub struct EventMarker {
    /// The collection's key (the hash of its full name).
    pub key: u32,
    /// The full name, for example `race_bin_01/epic_pursuit/win_nis_marker`.
    pub name: String,
    /// `Position` in physics space and `Rotation` in degrees (see [`super::placement`]).
    pub placement: Placement,
    /// `Dimensions` as stored, when the marker sets them.
    pub dimensions: Option<[f32; 3]>,
}

/// The markers of the final race. Each is `None` when the install does not have that child.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalRaceMarkers {
    /// `startgrid1` (class `startgrid`): the front of the start grid.
    pub start_grid: Option<EventMarker>,
    /// `win_trigger` (a box): the player wins by entering it at speed (`story-flow-2.md` section 5.3).
    pub win_trigger: Option<BoxTrigger>,
    /// `lose_trigger` (a box): entering it before a win totals the racer (`story-flow-2.md` F-32).
    pub lose_trigger: Option<BoxTrigger>,
    /// `barrier_area` (class `areaunlocktrigger`): the GPS target once the race has run 300 s (`story-flow-2.md` F-32).
    pub barrier_area: Option<EventMarker>,
    /// `win_nis_marker`: where the win cutscene `EndingNis04` plays (`story-flow-2.md` section 5.3).
    pub win_nis_marker: Option<EventMarker>,
    /// `taggedcarmarker1` to `taggedcarmarker6`, in order; a missing one is skipped.
    pub tagged_car_markers: Vec<EventMarker>,
}

/// Reads the final race's markers. `None` when the database has no `race_bin_01/epic_pursuit`.
pub(crate) fn final_race(db: &Database) -> Option<FinalRaceMarkers> {
    collection(db, vlt_hash(FINAL_RACE))?;
    Some(FinalRaceMarkers {
        start_grid: marker(db, "startgrid1"),
        win_trigger: box_trigger(db, "win_trigger"),
        lose_trigger: box_trigger(db, "lose_trigger"),
        barrier_area: marker(db, "barrier_area"),
        win_nis_marker: marker(db, "win_nis_marker"),
        tagged_car_markers: (1..=6).filter_map(|i| marker(db, &format!("taggedcarmarker{i}"))).collect(),
    })
}

/// The child `name` of the final race, by its collection key.
fn child<'a>(db: &'a Database, name: &str) -> Option<CollectionRef<'a>> {
    collection(db, vlt_hash(&format!("{FINAL_RACE}/{name}")))
}

fn marker(db: &Database, name: &str) -> Option<EventMarker> {
    let c = child(db, name)?;
    Some(EventMarker {
        key: c.key(),
        name: format!("{FINAL_RACE}/{name}"),
        placement: placement(c),
        dimensions: c.get_vector3("Dimensions"),
    })
}

fn box_trigger(db: &Database, name: &str) -> Option<BoxTrigger> {
    child(db, name).map(trigger)
}
