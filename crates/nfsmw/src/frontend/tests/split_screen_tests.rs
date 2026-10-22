//! The split-screen menu flow against the install (`docs/specs/split-screen.md` section 2): the mode select, the track list and
//! options, the Press Start screens that seat the devices, and the car selects of the two players. Skipped without
//! `NFSMW_GAME_DIR`. The screens share the custom race's state, so the tests take its turn.

/// The co-op career entry reuses this flow's Press Start screens (`docs/specs/coop-career.md` section 3).
mod coop_flow;

use super::custom_race_screen_tests::{TURN, install, open_in, read};
use super::*;
use crate::frontend::custom_race::settings::Row;
use crate::frontend::custom_race::{Mode, options, region, split, state, tracks};
use crate::frontend::icon_items::{Id, icons};
use crate::frontend::icon_menu::Kind;
use crate::frontend::logic::Sub;
use crate::frontend::quick_play::pool::PROLOGUE_STAGE;
use crate::frontend::quick_race;
use crate::input::PlayerDevice;
use crate::scenes::world::SplitSetup;
use crate::settings::Transmission;

const SPRINT: &str = "race_bin_10/10_2_1_sprint";
const DRAG: &str = "race_bin_02/2_1_2_drag";
const PLAYER_TEXT: u32 = 0x42EF_4C2E;
const HEADLINE: u32 = 0xB244_CF71;

/// Leaves the shared state as a custom race expects it, whatever a test does.
struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        state::with(|s| {
            s.split.end();
            s.split.accepted = [None, None];
            s.set_event(None);
        });
    }
}

/// Starts the two-player flow on a track, as the Split Screen icon and the track list do.
fn begin(mode: Mode, event: &str) -> Reset {
    state::with(|s| {
        s.split.begin();
        s.choose_mode(mode);
        s.reset_settings();
        s.set_event(Some(event.into()));
    });
    Reset
}

fn text_of(h: &Harness, name_hash: u32) -> Option<String> {
    let tree = h.screens.trees().pop()?;
    tree.nodes.iter().find(|n| n.name_hash == name_hash).and_then(|n| n.text.clone())
}

/// A Start press of `device` on this frame.
fn start(h: &mut Harness, device: PlayerDevice) {
    h.screens.set_start_press(Some(device));
    h.run(0, 1);
    h.wait(2.0);
}

fn press_start(player: u8) -> Option<Harness> {
    install()?;
    Harness::open(screen::PRESS_START, Args { player, ..Args::default() })
}

/// A player's car select in the two-player flow (`open_in` resets the mode's own state).
fn car_select(mode: Mode, event: &str, player: u8) -> Option<Harness> {
    state::with(|s| s.split.begin());
    let mut h = open_in(mode, screen::CAR_SELECT, Args { custom: true, player, ..Args::default() })?;
    state::with(|s| s.set_event(Some(event.into())));
    h.wait(1.0);
    Some(h)
}

#[test]
fn the_mode_select_offers_four_modes_in_split_screen_and_six_otherwise() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let modes = |on: bool| {
        state::with(|s| if on { s.split.begin() } else { s.split.end() });
        icons(Kind::CustomRace).into_iter().map(|i| i.id).collect::<Vec<_>>()
    };
    assert_eq!(
        modes(true),
        [Mode::Circuit, Mode::Sprint, Mode::Drag, Mode::SpeedTrap].map(Id::CustomMode),
        "no Lap Knockout and no Tollbooth"
    );
    assert_eq!(modes(false).len(), 6);
    assert!(icons(Kind::QuickRace).iter().all(|i| i.enabled), "Split Screen is no longer greyed out");
}

#[test]
fn the_split_screen_icon_sets_the_flag_and_opens_the_mode_select() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { sub: Sub::QuickRace, ..Args::default() }) else {
        return;
    };
    h.wait(1.5);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(2.0);
    assert!(state::with(|s| s.split.on), "the third icon sets two players");
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    // Back, then Custom Race: one player again.
    h.press(pad::BACK);
    h.wait(2.0);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.wait(2.0);
    assert!(!state::with(|s| s.split.on), "Custom Race is one player");
}

#[test]
fn a_drag_sprint_or_speed_trap_track_goes_straight_to_press_start_and_a_circuit_to_its_laps() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dir) = install() else { return };
    read(&dir);
    for (mode, wanted) in [
        (Mode::Drag, screen::PRESS_START),
        (Mode::Sprint, screen::PRESS_START),
        (Mode::SpeedTrap, screen::PRESS_START),
        (Mode::Circuit, screen::TRACK_OPTIONS),
    ] {
        let _reset = Reset;
        let Some(mut h) = open_in(mode, screen::TRACK_SELECT, Args::default()) else { return };
        state::with(|s| s.split.begin());
        let board = quick_race::board().expect("the board was read");
        let standing = tracks::Standing::at(PROLOGUE_STAGE);
        assert!(!tracks::list(&board, mode, region::Filter::ALL, &standing, true).is_empty(), "{mode:?}");
        // The first track of the list may be locked on a fresh profile: step to an open one.
        let open = tracks::list(&board, mode, region::Filter::ALL, &standing, tracks::unlock_all())
            .iter()
            .position(|listed| !listed.locked)
            .expect("an open track");
        for _ in 0..open {
            h.press(pad::RIGHT);
        }
        h.press(pad::ACCEPT);
        h.wait(3.0);
        assert_eq!(h.screens.top(), Some(wanted), "{mode:?}");
    }
}

#[test]
fn the_circuit_options_show_only_the_laps_and_accept_goes_to_press_start() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let Some(mut h) = open_in(Mode::Circuit, screen::TRACK_OPTIONS, Args::default()) else { return };
    state::with(|s| s.split.begin());
    assert_eq!(options::rows(), [Row::Laps], "no Traffic, Opponents, Difficulty, Catch Up or Heat Level");
    state::with(|s| s.set_event(Some(SPRINT.into())));
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PRESS_START), "no car select before Press Start");
}

#[test]
fn press_start_names_the_player() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = begin(Mode::Sprint, SPRINT);
    for (player, text) in [(0, "Player 1 press START."), (1, "Player 2 press START.")] {
        let Some(mut h) = press_start(player) else { return };
        h.wait(1.0);
        assert_eq!(text_of(&h, HEADLINE).as_deref(), Some(text));
    }
}

#[test]
fn the_device_that_presses_start_is_seated_and_the_car_select_opens() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = begin(Mode::Sprint, SPRINT);
    let Some(mut h) = press_start(0) else { return };
    h.wait(1.0);
    // The accept key alone seats nobody: the Start press of a device does.
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PRESS_START));
    start(&mut h, PlayerDevice::KeyboardArrows);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT));
    assert_eq!(state::with(|s| s.split.device(0)), Some(PlayerDevice::KeyboardArrows));
}

#[test]
fn player_two_cannot_take_player_ones_device() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = begin(Mode::Sprint, SPRINT);
    state::with(|s| s.split.seat(0, PlayerDevice::Keyboard));
    let Some(mut h) = press_start(1) else { return };
    h.wait(1.0);
    start(&mut h, PlayerDevice::Keyboard);
    assert_eq!(h.screens.top(), Some(screen::PRESS_START), "the keyboard is player 1's");
    assert_eq!(state::with(|s| s.split.device(1)), None);
    start(&mut h, PlayerDevice::KeyboardArrows);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT));
    assert_eq!(state::with(|s| s.split.seated()), Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]));
}

#[test]
fn back_from_press_start_follows_the_chain() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    if install().is_none() {
        return;
    }
    // Player 1 returns to the options, or the track list for a race that has no options.
    for (mode, back_to) in [(Mode::Circuit, screen::TRACK_OPTIONS), (Mode::Drag, screen::TRACK_SELECT)] {
        let _reset = begin(mode, SPRINT);
        let Some(mut h) = press_start(0) else { return };
        h.wait(1.0);
        h.press(pad::BACK);
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(back_to), "{mode:?}");
    }
    // Player 2 returns to player 1's car select.
    let _reset = begin(Mode::Circuit, SPRINT);
    let Some(mut h) = press_start(1) else { return };
    h.wait(1.0);
    h.press(pad::BACK);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::CAR_SELECT));
    assert_eq!(text_of(&h, PLAYER_TEXT).as_deref(), Some("Player 1"));
}

#[test]
fn the_car_select_names_the_player_and_remembers_both_picks() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    for (player, name) in [(0, "Player 1"), (1, "Player 2")] {
        let Some(mut h) = car_select(Mode::Sprint, SPRINT, player) else { return };
        assert_eq!(text_of(&h, PLAYER_TEXT).as_deref(), Some(name));
        // Down changes the list, Right the car: the pick is this player's.
        h.press(pad::DOWN);
        h.press(pad::RIGHT);
    }
    let [first, second] = state::with(|s| [s.split.cars[0].name.clone(), s.split.cars[1].name.clone()]);
    assert!(first.is_some() && second.is_some(), "{first:?} {second:?}");
    // A player opening their select again finds their own car.
    let Some(h) = car_select(Mode::Sprint, SPRINT, 0) else { return };
    assert_eq!(state::with(|s| s.pick(0).name.clone()), first);
    drop(h);
}

#[test]
fn back_from_a_car_select_returns_to_that_players_press_start() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    for player in [0u8, 1] {
        let Some(mut h) = car_select(Mode::Sprint, SPRINT, player) else { return };
        state::with(|s| {
            s.split.seat(0, PlayerDevice::Keyboard);
            s.split.seat(1, PlayerDevice::KeyboardArrows);
        });
        h.press(pad::BACK);
        h.wait(3.0);
        assert_eq!(h.screens.top(), Some(screen::PRESS_START), "player {}", player + 1);
        h.wait(1.0);
        assert_eq!(text_of(&h, HEADLINE), Some(format!("Player {} press START.", player + 1)));
        let second = state::with(|s| s.split.device(1));
        assert_eq!(second, (player == 0).then_some(PlayerDevice::KeyboardArrows), "player 2's seat is given up");
    }
}

/// Accepts the shown car and answers the transmission question with the first answer (Manual).
fn accept_and_answer(h: &mut Harness) {
    h.press(pad::ACCEPT);
    h.wait(3.0);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "the question: {:?}", h.commands);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
}

#[test]
fn player_ones_car_asks_for_the_gearbox_and_goes_on_to_player_twos_press_start() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let Some(mut h) = car_select(Mode::Sprint, SPRINT, 0) else { return };
    state::with(|s| s.split.seat(0, PlayerDevice::Keyboard));
    accept_and_answer(&mut h);
    let said = h.commands.iter().find_map(|c| match c {
        Command::SplitFirstCar(t) => Some(*t),
        _ => None,
    });
    assert!(said.is_some(), "{:?}", h.commands);
    assert!(!h.commands.iter().any(|c| matches!(c, Command::StartSplitRace { .. })), "player 2 has not chosen");
    assert!(state::with(|s| s.split.accepted[0].is_some()), "the accepted car is kept for the world");
}

#[test]
fn player_twos_car_starts_the_race_with_both_players() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let Some(mut h) = car_select(Mode::Circuit, SPRINT, 1) else { return };
    state::with(|s| {
        s.settings_mut().laps = 3;
        s.split.seat(0, PlayerDevice::Keyboard);
        s.split.set_transmission(0, Transmission::Manual);
        s.split.seat(1, PlayerDevice::KeyboardArrows);
        s.split.accepted[0] = Some("BMWM3GTRE46:M3GTRCAREERSTART".into());
    });
    accept_and_answer(&mut h);
    let started = h.commands.iter().find_map(|c| match c {
        Command::StartSplitRace { race, options, setup } => Some((race.clone(), *options, setup.clone())),
        _ => None,
    });
    let (race, options, setup): (String, _, SplitSetup) = started.unwrap_or_else(|| panic!("{:?}", h.commands));
    assert_eq!(race, SPRINT);
    assert_eq!(
        (options.opponents, options.heat, options.catch_up, options.laps),
        (Some(1), Some(0), Some(false), Some(3))
    );
    assert_eq!(setup.devices, [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]);
    assert_eq!(setup.cars[0], "BMWM3GTRE46:M3GTRCAREERSTART");
    assert!(!setup.cars[1].is_empty());
    assert_eq!(setup.transmissions[0], Transmission::Manual, "player 1's gearbox is their own answer");
}

#[test]
fn a_drag_race_skips_the_gearbox_question_for_both_players() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    for player in [0u8, 1] {
        let Some(mut h) = car_select(Mode::Drag, DRAG, player) else { return };
        state::with(|s| {
            s.split.seat(0, PlayerDevice::Keyboard);
            s.split.seat(1, PlayerDevice::KeyboardArrows);
            s.split.accepted[0] = Some("BMWM3GTRE46".into());
        });
        h.press(pad::ACCEPT);
        h.wait(3.0);
        assert_ne!(h.screens.top(), Some(screen::DIALOG), "player {}: {:?}", player + 1, h.commands);
        let wanted = |c: &Command| match player {
            0 => matches!(c, Command::SplitFirstCar(_)),
            _ => matches!(c, Command::StartSplitRace { .. }),
        };
        assert!(h.commands.iter().any(wanted), "player {}: {:?}", player + 1, h.commands);
    }
}

#[test]
fn only_the_circuit_keeps_an_options_page() {
    assert!(split::skips_options(Mode::Drag));
    assert!(!split::skips_options(Mode::Circuit));
}
