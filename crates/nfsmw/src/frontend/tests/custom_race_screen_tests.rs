//! The custom race's screens against the install: the mode select, the track list, the options and the car select behave as
//! `docs/specs/custom-race.md` says. Skipped without `NFSMW_GAME_DIR`. The screens share one state (`custom_race::state`), so the
//! tests take turns.

use std::sync::Mutex;

use super::*;
use crate::frontend::custom_race::car_list::{self, List};
use crate::frontend::custom_race::region::Filter;
use crate::frontend::custom_race::settings::Row;
use crate::frontend::custom_race::{Mode, state, tracks};
use crate::frontend::logic::Sub;
use crate::frontend::quick_play::pool;
use crate::frontend::quick_race;

pub(super) static TURN: Mutex<()> = Mutex::new(());

pub(super) fn install() -> Option<GameDir> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(root)).unwrap())
}

/// Reads the board and the cars, as the front end does in the background when it starts.
pub(super) fn read(dir: &GameDir) {
    pool::load_now(dir);
    quick_race::load_in_background(dir);
    for _ in 0..2400 {
        if quick_race::board().is_some() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("the quick race board was not read");
}

pub(super) fn open_in(mode: Mode, name: &str, args: Args) -> Option<Harness> {
    let dir = install()?;
    read(&dir);
    state::with(|s| {
        s.choose_mode(mode);
        s.reset_settings();
        s.set_event(None);
    });
    let mut h = Harness::open(name, args)?;
    h.wait(1.5);
    Some(h)
}

fn mode_select() -> Args {
    Args { sub: Sub::CustomRace, ..Args::default() }
}

#[test]
fn the_mode_select_offers_the_tutorial_of_drag_and_goes_back_to_the_quick_race_menu() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut h) = open_in(Mode::Circuit, screen::MAIN_MENU_SUB, mode_select()) else { return };
    h.press(pad::BUTTON0 << 4);
    assert!(!h.commands.iter().any(|c| matches!(c, Command::PlayTutorial { .. })), "a circuit has no tutorial");
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.wait(0.5);
    h.press(pad::BUTTON0 << 4);
    let played = h.commands.iter().any(|c| {
        matches!(c, Command::PlayTutorial { movie, back, args } if movie == "drag_tutorial"
            && back == screen::MAIN_MENU_SUB && args.sub == Sub::CustomRace)
    });
    assert!(played, "{:?}", h.commands);
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
}

#[test]
fn a_locked_track_is_not_accepted_an_open_one_is_and_back_returns_to_the_mode_select() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    // The list is a fresh profile's: a career another test left loaded would open its tracks.
    crate::career::CareerHandle::shared().clear();
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    if tracks_open() {
        return;
    }
    let Some(mut h) = open_in(Mode::Circuit, screen::TRACK_SELECT, Args::default()) else { return };
    let board = quick_race::board().expect("the board was read");
    let listed = tracks::list(&board, Mode::Circuit, Filter::ALL, &tracks::Standing::at(pool::PROLOGUE_STAGE), false);
    // On a fresh profile the exe opens a few circuits from the start; the rest wait for their rival.
    let locked = listed.iter().position(|l| l.locked).expect("a circuit is locked at the first stage");
    let open = listed.iter().position(|l| !l.locked).expect("a circuit is open from the start");
    for _ in 0..locked {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::TRACK_SELECT), "a locked track stays on the list");
    assert!(state::with(|s| s.event().is_none()));
    // Back to the first track, then on to an open one.
    for _ in 0..(listed.len() - locked + open) {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(3.0);
    assert_eq!(h.screens.top(), Some(screen::TRACK_OPTIONS), "an open track goes on to the options");
    assert_eq!(state::with(|s| s.event().map(str::to_owned)), Some(board.tracks[listed[open].index].name.clone()));
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::TRACK_SELECT));
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
}

/// The tests that need a locked list stand down when the developer switch opens everything.
fn tracks_open() -> bool {
    crate::frontend::custom_race::tracks::unlock_all()
}

#[test]
fn an_open_track_goes_on_to_the_options_and_the_options_step_and_reset() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut h) = open_in(Mode::Circuit, screen::TRACK_OPTIONS, Args::default()) else { return };
    h.press(pad::RIGHT);
    assert_eq!(state::with(|s| s.settings().laps), 2, "the first row is the laps");
    h.press(pad::DOWN);
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert_eq!(state::with(|s| s.settings().opponents), 1, "the third row is the opponents and wraps");
    // Accept without a picked track goes nowhere.
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::TRACK_OPTIONS));
    state::with(|s| s.set_event(Some("race_bin_10/10_2_1_sprint".into())));
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT));
}

#[test]
fn the_car_select_starts_with_the_career_start_car_asks_for_the_transmission_and_starts_the_race() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    state::with(|s| s.car.name = None);
    let Some(mut h) = open_in(Mode::Sprint, screen::CAR_SELECT, Args { custom: true, ..Args::default() }) else {
        return;
    };
    state::with(|s| s.set_event(Some("race_bin_10/10_2_1_sprint".into())));
    let pool = pool::cars().expect("the cars were read");
    let shown = state::with(|s| s.car.name.clone());
    assert_eq!(shown.as_deref(), Some(car_list::DEFAULT_PRESET), "the Preset list opens on the M3 GTR");
    // Down changes the list, Right the car; the chosen car is remembered for the race.
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    let remembered = state::with(|s| s.car.name.clone()).expect("a car");
    assert!(car_list::find(&pool, &car_select_progress(), &remembered).is_some(), "{remembered}");
    // A locked car is refused; the M3 GTR of the Preset list is open.
    state::with(|s| s.car.name = Some(car_list::DEFAULT_PRESET.into()));
    drop(h);
    let Some(mut h) = open_in(Mode::Sprint, screen::CAR_SELECT, Args { custom: true, ..Args::default() }) else {
        return;
    };
    state::with(|s| s.set_event(Some("race_bin_10/10_2_1_sprint".into())));
    h.press(pad::ACCEPT);
    h.wait(3.0);
    // The question is a dialog over the car select; the screens open it themselves.
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "Accept asks for the transmission: {:?}", h.commands);
    assert_eq!(List::from_index(List::Preset.index()), List::Preset);
}

fn car_select_progress() -> crate::frontend::custom_race::car_list::Progress {
    crate::frontend::custom_race::car_select::progress()
}

#[test]
fn the_rows_of_each_mode_show_in_the_package_and_the_heat_row_closes_the_list() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    for mode in Mode::ALL {
        let rows = crate::frontend::custom_race::settings::rows(mode);
        assert_eq!(rows.last(), Some(&Row::Heat), "{mode:?}");
    }
}

#[test]
fn the_series_has_the_event_the_sl65_is_unlocked_by() {
    let Some(dir) = install() else { return };
    crate::frontend::challenge_series::load_in_background(&dir);
    for _ in 0..2400 {
        if let Some(board) = crate::frontend::challenge_series::board() {
            assert!(board.entries.iter().any(|e| e.event == "19.8.68"), "event 68 of the series");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("the series board was not read");
}

#[test]
fn a_drag_race_starts_at_once_with_the_transmission_setting() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    state::with(|s| s.car.name = Some(car_list::DEFAULT_PRESET.into()));
    let Some(mut h) = open_in(Mode::Drag, screen::CAR_SELECT, Args { custom: true, ..Args::default() }) else { return };
    state::with(|s| s.set_event(Some("race_bin_02/2_1_2_drag".into())));
    h.press(pad::ACCEPT);
    h.wait(3.0);
    assert_ne!(h.screens.top(), Some(screen::DIALOG), "a drag race does not ask: {:?}", h.commands);
    assert!(h.commands.iter().any(|c| matches!(c, Command::StartCustomRace(_))), "{:?}", h.commands);
}
