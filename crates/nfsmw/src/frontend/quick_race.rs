//! Quick Race: the list of tracks the player can pick from and the pick itself. The list is the races of the install's
//! gameplay data; it is read on a worker thread when the front end starts, so the track screen has it by the time it is
//! opened (`track_select.rs`). Spec: `docs/specs/frontend-menus.md` section 5, `docs/specs/race-mode.md` section 11.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use game_install::GameDir;
use nfsmw_data::gameplay::{GameplayData, Race, RaceType};

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
}

/// The tracks, in the order the screen steps through them.
#[derive(Debug, Default)]
pub struct Board {
    pub tracks: Vec<Track>,
}

static BOARD: OnceLock<Arc<Board>> = OnceLock::new();
static PICK: AtomicUsize = AtomicUsize::new(0);

/// Starts reading the track list in the background. Does nothing when it was started already.
pub fn load_in_background(dir: &GameDir) {
    if BOARD.get().is_some() {
        return;
    }
    let dir = dir.clone();
    let spawned = std::thread::Builder::new().name("quick-race-board".into()).spawn(move || {
        let board = match nfsmw_data::gameplay::load(&dir) {
            Ok(data) => Board::from_data(&data),
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

/// The index of the picked track.
pub fn pick() -> usize {
    PICK.load(Ordering::Relaxed)
}

/// Moves the pick by `step` (wrapping) and returns the track now picked.
pub fn step(board: &Board, step: i32) -> Option<&Track> {
    let count = board.tracks.len() as i32;
    if count == 0 {
        return None;
    }
    let next = (pick() as i32 + step).rem_euclid(count) as usize;
    PICK.store(next, Ordering::Relaxed);
    board.tracks.get(next)
}

/// The track that is picked.
pub fn picked(board: &Board) -> Option<&Track> {
    board.tracks.get(pick().min(board.tracks.len().saturating_sub(1)))
}

impl Board {
    /// The races a quick race can run: every type the world runs except the pursuit challenges, by event id.
    pub fn from_data(data: &GameplayData) -> Self {
        let mut tracks: Vec<(Vec<u32>, Track)> = data
            .races
            .iter()
            .filter(|r| matches!(r.race_type, Some(t) if t != RaceType::Challenge))
            .filter(|r| r.race_type.is_some_and(|t| t != RaceType::CheckpointRace) && r.start.is_some())
            .filter(|r| r.finish.is_some() || !r.checkpoints.is_empty())
            .map(|r| (event_key(&r.name), track_of(data, r)))
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

fn kind_label(kind: RaceType) -> &'static str {
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

fn track_of(data: &GameplayData, race: &Race) -> Track {
    let kind = race.race_type.map_or("Race", kind_label);
    let centres: Vec<glam::Vec3> = race
        .start
        .iter()
        .map(|s| s.position)
        .chain(race.checkpoints.iter().filter_map(|k| data.trigger(*k)).map(|t| t.position))
        .chain(race.finish.iter().filter_map(|k| data.trigger(*k)).map(|t| t.position))
        .collect();
    let length: f32 = centres.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    Track { name: race.name.clone(), title: format!("{kind} {}", crate::career::event_id(&race.name)), kind, length }
}

impl Track {
    /// The length as the screen shows it: kilometres with one decimal, or metres under a kilometre.
    pub fn length_text(&self) -> String {
        match self.length {
            m if m >= 1000.0 => format!("{:.1}KM", m / 1000.0),
            m => format!("{m:.0}M"),
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
        let board = Board::from_data(&data);
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

    #[test]
    fn a_length_reads_in_kilometres_or_metres() {
        let track = |length| Track { name: String::new(), title: String::new(), kind: "Sprint", length };
        assert_eq!(track(7486.8).length_text(), "7.5KM");
        assert_eq!(track(640.0).length_text(), "640M");
    }

    #[test]
    fn stepping_wraps_round_the_list() {
        let board = Board {
            tracks: ["a", "b", "c"]
                .iter()
                .map(|n| Track { name: (*n).into(), title: String::new(), kind: "Sprint", length: 0.0 })
                .collect(),
        };
        PICK.store(0, Ordering::Relaxed);
        assert_eq!(step(&board, -1).map(|t| t.name.as_str()), Some("c"));
        assert_eq!(step(&board, 1).map(|t| t.name.as_str()), Some("a"));
        assert_eq!(step(&board, 4).map(|t| t.name.as_str()), Some("b"));
        assert_eq!(picked(&board).map(|t| t.name.as_str()), Some("b"));
        assert_eq!(step(&Board::default(), 1), None);
        PICK.store(0, Ordering::Relaxed);
    }
}
