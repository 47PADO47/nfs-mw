//! The entries of the series: the races of the challenge bin (bin 19), in the order the bin lists them. They are read
//! on a worker thread when the front end starts, so the screen has them when it opens (as `quick_race.rs` does).

use std::sync::{Arc, OnceLock};

use blackbox_feng::fe_hash_upper;
use blackbox_minimap::Calibration;
use game_install::GameDir;
use nfsmw_data::gameplay::{GameplayData, Race};

use super::kinds::Kind;
use super::view::Crop;

/// The bin whose world races are the series (`UIQRChallengeSeries::Setup` asks for bin `0x13`).
pub const BIN: i32 = 19;

/// One event of the series.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The race's collection key, which an `UnlockRaces` list names.
    pub key: u32,
    /// The race's collection name, what a race start takes.
    pub name: String,
    /// The event id, for example `19.8.32`.
    pub event: String,
    pub kind: Kind,
    /// `LocalizationTag`: the number the event's texts are named by.
    pub tag: i32,
    /// `ThreshholdValue`: what a pursuit's statistic has to reach.
    pub goal: f32,
    /// The event is open from the start (`InitiallyUnlocked`).
    pub open_at_start: bool,
    /// The keys of the events a win opens (`UnlockRaces`).
    pub unlocks: Vec<u32>,
    /// The part of the map picture the screen zooms to.
    pub crop: Crop,
    /// The map picture: the texture `TRACK_MAP_<event id>` of `TrackMaps.bin`.
    pub map_texture: u32,
}

/// The events, in the order the screen lists them.
#[derive(Debug, Default, PartialEq)]
pub struct Board {
    pub entries: Vec<Entry>,
}

static BOARD: OnceLock<Arc<Board>> = OnceLock::new();
static INSTALL: OnceLock<GameDir> = OnceLock::new();

/// The install the events were read from, for the map pictures the screen reads when it opens.
pub fn install() -> Option<&'static GameDir> {
    INSTALL.get()
}

/// Starts reading the events in the background. Does nothing when it was started already.
pub fn load_in_background(dir: &GameDir) {
    if BOARD.get().is_some() {
        return;
    }
    let _ = INSTALL.set(dir.clone());
    let dir = dir.clone();
    let spawned = std::thread::Builder::new().name("challenge-series-board".into()).spawn(move || {
        let board = read(&dir).unwrap_or_else(|e| {
            log::warn!("challenge series: no event list: {e:#}");
            Board::default()
        });
        log::info!("challenge series: {} event(s)", board.entries.len());
        let _ = BOARD.set(Arc::new(board));
    });
    if let Err(e) = spawned {
        log::warn!("challenge series: cannot read the event list: {e}");
    }
}

/// The events, once they have been read.
pub fn board() -> Option<Arc<Board>> {
    BOARD.get().cloned()
}

fn read(dir: &GameDir) -> anyhow::Result<Board> {
    let data = nfsmw_data::gameplay::load(dir)?;
    let map = nfsmw_data::minimap::open_city_calibration(dir)?;
    Ok(Board::from_data(&data, &map))
}

impl Board {
    /// The races of bin 19 in the bin's order. A race the data does not hold is skipped.
    pub fn from_data(data: &GameplayData, map: &Calibration) -> Self {
        let Some(bin) = data.bins.iter().find(|b| b.number == BIN) else { return Self::default() };
        let races = bin.world_races.iter().filter_map(|key| data.races.iter().find(|r| r.key == *key));
        Self { entries: races.map(|race| entry_of(data, race, map)).collect() }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

fn entry_of(data: &GameplayData, race: &Race, map: &Calibration) -> Entry {
    let event = crate::career::race_event_id(race);
    let p = &race.params;
    Entry {
        key: race.key,
        name: race.name.clone(),
        kind: Kind::of(p.milestone.as_deref()),
        tag: p.localization_tag,
        goal: p.threshold,
        open_at_start: p.initially_unlocked,
        unlocks: p.unlock_races.clone(),
        crop: Crop::around(&route_on_map(data, race, map)),
        map_texture: map_texture(&event),
        event,
    }
}

/// The name of the map picture of an event: `TRACK_MAP_` and the event id with its dots turned to underscores.
pub fn map_texture(event: &str) -> u32 {
    fe_hash_upper(&format!("TRACK_MAP_{}", event.replace('.', "_")))
}

/// The points of a race's route on the picture (`GRaceParameters::GetBoundingBox` is the box of these): the start, the
/// checkpoints and the finish, from physics space to the map's `0..1` units.
fn route_on_map(data: &GameplayData, race: &Race, map: &Calibration) -> Vec<[f32; 2]> {
    let triggers = race.checkpoints.iter().chain(race.finish.iter()).filter_map(|key| data.trigger(*key));
    let points = race.start.iter().map(|s| s.position).chain(triggers.map(|t| t.position));
    // Physics space is x right, y up, z forward; the map's world is x forward, y left: `(z, -x)`.
    points.map(|p| map.to_map([p.z, -p.x])).collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use nfsmw_data::gameplay::{Bin, BoxTrigger, Placement, RaceParams, RaceType};

    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    fn trigger(key: u32, at: Vec3) -> BoxTrigger {
        BoxTrigger {
            key,
            name: String::new(),
            position: at,
            heading_deg: 0.0,
            dimensions: [2.0, 40.0, 40.0],
            one_shot: false,
            fire_on_exit: false,
            particle_effect: None,
            flare_spacing: None,
            time_bonus: 0.0,
            resets_player: false,
        }
    }

    fn race(key: u32, event: &str, milestone: Option<&str>, goal: f32) -> Race {
        Race {
            key,
            name: format!("race_bin_challenge/{event}"),
            event: Some(event.into()),
            event_icon: String::new(),
            race_type: Some(RaceType::Challenge),
            region: None,
            laps: 1,
            start: Some(Placement { position: Vec3::new(0.0, 0.0, 0.0), heading_deg: 0.0 }),
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams {
                milestone: milestone.map(str::to_owned),
                threshold: goal,
                localization_tag: 7,
                ..RaceParams::default()
            },
        }
    }

    #[test]
    fn the_bins_order_gives_the_screens_order_and_each_entry_carries_its_rules() {
        let mut first = race(1, "19.8.2", Some("bounty_in_pursuit"), 1000.0);
        first.params.initially_unlocked = true;
        first.params.unlock_races = vec![2];
        let data = GameplayData {
            races: vec![race(2, "19.8.1", None, 0.0), first],
            bins: vec![Bin { number: BIN, world_races: vec![1, 2], ..Bin::default() }],
            ..GameplayData::default()
        };
        let board = Board::from_data(&data, &CITY);
        let events: Vec<&str> = board.entries.iter().map(|e| e.event.as_str()).collect();
        assert_eq!(events, ["19.8.2", "19.8.1"], "the bin's order, not the event numbers'");
        assert!(board.entries[0].open_at_start);
        assert_eq!(board.entries[0].unlocks, [2]);
        assert_eq!(board.entries[0].goal, 1000.0);
        assert_eq!(board.entries[0].tag, 7);
        assert_eq!(board.entries[1].kind, Kind::Race);
        assert_eq!(board.entries[0].map_texture, fe_hash_upper("TRACK_MAP_19_8_2"));
    }

    #[test]
    fn a_data_without_the_bin_has_no_entries() {
        assert_eq!(Board::from_data(&GameplayData::default(), &CITY).len(), 0);
    }

    #[test]
    fn a_route_crop_holds_the_start_checkpoints_and_finish() {
        let mut r = race(1, "19.8.32", None, 0.0);
        r.start = Some(Placement { position: Vec3::new(0.0, 0.0, 0.0), heading_deg: 0.0 });
        r.checkpoints = vec![10];
        r.finish = Some(11);
        // Physics (x, y, z) = (-north, up, east): a point 1332 m east and 1332 m north of the start.
        let data = GameplayData {
            triggers: vec![trigger(10, Vec3::new(-1332.0, 0.0, 0.0)), trigger(11, Vec3::new(-1332.0, 0.0, 1332.0))],
            ..GameplayData::default()
        };
        let points = route_on_map(&data, &r, &CITY);
        assert_eq!(points.len(), 3);
        let crop = Crop::around(&points);
        assert!(crop.half > 0.125, "a route longer than a point is wider than the margin alone: {crop:?}");
    }

    #[test]
    fn the_map_texture_names_replace_the_dots() {
        assert_eq!(map_texture("19.9.70"), fe_hash_upper("TRACK_MAP_19_9_70"));
    }
}
