//! Quick Race: the list of tracks the player can pick from and the pick itself. The list is the races of the install's
//! gameplay data; it is read on a worker thread when the front end starts, so the track screen has it by the time it is
//! opened (`track_select.rs`). Spec: `docs/specs/frontend-menus.md` section 5, `docs/specs/race-mode.md` section 11.

use std::sync::{Arc, OnceLock};

use blackbox_minimap::Calibration;
use game_install::GameDir;
use nfsmw_data::gameplay::{GameplayData, Race, RaceType};

use super::challenge_series::{Crop, map_texture, route_on_map};
use super::custom_race::Region;

/// One track of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// The race's collection name, what `race start` takes.
    pub name: String,
    /// The line the screen shows: the kind and the event, for example `Sprint 10.2.1`.
    pub title: String,
    /// The race kind as the screen's second line shows it.
    pub kind: &'static str,
    /// The route length in metres, from the checkpoints and the finish (0 when there is none).
    pub length: f32,
    /// The race type, which picks the Quick Play brief's icon and mode name.
    pub race_type: RaceType,
    /// `RaceLength`: the route in metres as the data stores it, 0 when it has none (`brief_length`).
    pub route: f32,
    /// The laps the race runs with its own values (`NumLaps`, at least one).
    pub laps: u32,
    /// The opponents the race lists (`Opponents`); the number a race starts with unless the player sets another.
    pub opponents: usize,
    /// The Blacklist bin of the race's event id (`10.2.1` is bin 10), `None` for a race outside bins 1 to 15.
    pub bin: Option<u8>,
    /// The event id the language table and the map picture are named by (`10.2.1`).
    pub event: String,
    /// The race's region, `None` when the data names none the front end knows.
    pub region: Option<Region>,
    /// The race is open from the start of a career (`InitiallyUnlocked`).
    pub open_at_start: bool,
    /// The part of the map picture the track list zooms to, and the picture's texture (`TRACK_MAP_<event id>`).
    pub crop: Crop,
    pub map_texture: u32,
}

/// The tracks, in the order the screen steps through them.
#[derive(Debug, Default)]
pub struct Board {
    pub tracks: Vec<Track>,
}

static BOARD: OnceLock<Arc<Board>> = OnceLock::new();

/// Starts reading the track list in the background. Does nothing when it was started already.
pub fn load_in_background(dir: &GameDir) {
    if BOARD.get().is_some() {
        return;
    }
    let dir = dir.clone();
    let spawned = std::thread::Builder::new().name("quick-race-board".into()).spawn(move || {
        let map = nfsmw_data::minimap::open_city_calibration(&dir).ok();
        let board = match nfsmw_data::gameplay::load(&dir) {
            Ok(data) => Board::from_data(&data, map.as_ref()),
            Err(e) => {
                log::warn!("quick race: no track list: {e:#}");
                Board::default()
            }
        };
        log::info!("quick race: {} track(s)", board.tracks.len());
        let _ = BOARD.set(Arc::new(board));
    });
    if let Err(e) = spawned {
        log::warn!("quick race: cannot read the track list: {e}");
    }
}

/// The track list, once it has been read.
pub fn board() -> Option<Arc<Board>> {
    BOARD.get().cloned()
}

impl Board {
    /// The races a quick race can run: every type the world runs except the pursuit challenges, by event id.
    pub fn from_data(data: &GameplayData, map: Option<&Calibration>) -> Self {
        let mut tracks: Vec<(Vec<u32>, Track)> = data
            .races
            .iter()
            .filter(|r| matches!(r.race_type, Some(t) if t != RaceType::Challenge))
            .filter(|r| r.race_type.is_some_and(|t| t != RaceType::CheckpointRace) && r.start.is_some())
            .filter(|r| r.finish.is_some() || !r.checkpoints.is_empty())
            .map(|r| (event_key(&r.name), track_of(data, r, map)))
            .collect();
        tracks.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
        Self { tracks: tracks.into_iter().map(|(_, t)| t).collect() }
    }
}

/// The numbers of an event id (`10.2.1` from `race_bin_10/10_2_1_sprint`), to order the list by.
fn event_key(name: &str) -> Vec<u32> {
    let id = crate::career::event_id(name);
    id.split('.').map(|part| part.parse().unwrap_or(u32::MAX)).collect()
}

pub(super) fn kind_label(kind: RaceType) -> &'static str {
    match kind {
        RaceType::Sprint => "Sprint",
        RaceType::Circuit => "Circuit",
        RaceType::Drag => "Drag",
        RaceType::Knockout => "Circuit - K.O.",
        RaceType::Tollbooth => "Tollbooth",
        RaceType::SpeedTrap => "Speedtrap",
        _ => "Race",
    }
}

fn track_of(data: &GameplayData, race: &Race, map: Option<&Calibration>) -> Track {
    let kind = race.race_type.map_or("Race", kind_label);
    let centres: Vec<glam::Vec3> = race
        .start
        .iter()
        .map(|s| s.position)
        .chain(race.checkpoints.iter().filter_map(|k| data.trigger(*k)).map(|t| t.position))
        .chain(race.finish.iter().filter_map(|k| data.trigger(*k)).map(|t| t.position))
        .collect();
    let length: f32 = centres.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    Track {
        name: race.name.clone(),
        title: format!("{kind} {}", crate::career::event_id(&race.name)),
        kind,
        length,
        race_type: race.race_type.unwrap_or(RaceType::Sprint),
        route: race.params.route_length,
        laps: u32::try_from(race.laps).unwrap_or(1).max(1),
        opponents: race.params.opponents.len(),
        bin: event_key(&race.name).first().and_then(|bin| u8::try_from(*bin).ok()).filter(|bin| (1..=15).contains(bin)),
        event: crate::career::event_id(&race.name),
        region: race.region.as_deref().and_then(Region::of),
        open_at_start: race.params.initially_unlocked,
        crop: map.map_or(Crop::WHOLE, |map| Crop::around(&route_on_map(data, race, map))),
        map_texture: map_texture(&crate::career::event_id(&race.name)),
    }
}

impl Track {
    /// The length the Quick Play brief shows, in metres: the stored `RaceLength`, else the length of the route.
    pub fn brief_length(&self) -> f32 {
        match self.route > 0.0 {
            true => self.route,
            false => self.length,
        }
    }
}

impl Track {
    /// A track with the given name and nothing else, for tests.
    #[cfg(test)]
    pub(super) fn sample(name: &str) -> Track {
        Track {
            name: name.into(),
            title: String::new(),
            kind: "Sprint",
            length: 0.0,
            race_type: RaceType::Sprint,
            route: 0.0,
            laps: 1,
            opponents: 0,
            bin: None,
            event: String::new(),
            region: None,
            open_at_start: false,
            crop: Crop::WHOLE,
            map_texture: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use nfsmw_data::gameplay::{BoxTrigger, Placement, RaceParams};

    fn race(name: &str, kind: RaceType) -> Race {
        Race {
            key: 1,
            name: name.into(),
            event: None,
            event_icon: String::new(),
            race_type: Some(kind),
            region: None,
            laps: 1,
            start: Some(Placement { position: Vec3::ZERO, heading_deg: 0.0 }),
            finish: Some(7),
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams::default(),
        }
    }

    fn finish_at(z: f32) -> BoxTrigger {
        BoxTrigger {
            key: 7,
            name: "finish".into(),
            position: Vec3::new(0.0, 0.0, z),
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

    #[test]
    fn the_board_orders_by_event_id_and_leaves_the_challenges_out() {
        let data = GameplayData {
            races: vec![
                race("race_bin_10/10_2_1_sprint", RaceType::Sprint),
                race("race_bin_02/2_1_1_circuit", RaceType::Circuit),
                race("race_bin_challenge/19_8_1_challenge", RaceType::Challenge),
                race("race_bin_02/2_1_10_drag", RaceType::Drag),
                race("race_bin_02/2_1_2_drag", RaceType::Drag),
            ],
            triggers: vec![finish_at(2500.0)],
            ..Default::default()
        };
        let board = Board::from_data(&data, None);
        let names: Vec<&str> = board.tracks.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "race_bin_02/2_1_1_circuit",
                "race_bin_02/2_1_2_drag",
                "race_bin_02/2_1_10_drag",
                "race_bin_10/10_2_1_sprint"
            ]
        );
        assert_eq!(board.tracks[0].title, "Circuit 2.1.1");
        assert_eq!(board.tracks[0].length, 2500.0);
    }
}
