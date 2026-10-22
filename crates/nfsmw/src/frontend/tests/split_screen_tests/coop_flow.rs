//! The co-op career's entry against the install (`docs/specs/coop-career.md` section 3): Press Start for player 2 from the
//! career menu, the career car select that picks player 2's car, and the career resumed with player 2 in it. Skipped
//! without `NFSMW_GAME_DIR`. The custom race's split state and the career handle are shared, so the tests take the custom
//! race's turn and put both back.

use super::super::custom_race_screen_tests::{TURN, install};
use super::super::*;
use crate::career::{CareerHandle, coop_players, set_coop_players};
use crate::frontend::custom_race::state;
use crate::frontend::flow_movie_tests::install_rig;
use crate::input::PlayerDevice;
use crate::settings::Transmission;

/// Leaves the shared two-player state empty, whatever a test does.
struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        state::with(|s| {
            s.split.end();
            s.split.accepted = [None, None];
        });
    }
}

/// Puts the co-op session and the career's safe house flag back as they were before the test.
struct CoopOff;

impl Drop for CoopOff {
    fn drop(&mut self) {
        set_coop_players(1);
        CareerHandle::shared().set_prelude(false);
    }
}

/// Press Start for player 2 of a co-op career, as the career menu's Co-op Career opens it, with the seats empty.
fn co_op_press_start() -> Option<Harness> {
    install()?;
    state::with(|s| s.split.clear_seats());
    Harness::open(screen::PRESS_START, Args { player: 1, coop: true, ..Args::default() })
}

/// A Start press of `device` on this frame, then the screens' transition.
fn start(h: &mut Harness, device: PlayerDevice) {
    h.screens.set_start_press(Some(device));
    h.run(0, 1);
    h.wait(2.0);
}

fn text_of(h: &Harness, name_hash: u32) -> Option<String> {
    let tree = h.screens.trees().pop()?;
    tree.nodes.iter().find(|n| n.name_hash == name_hash).and_then(|n| n.text.clone())
}

#[test]
fn co_op_press_start_seats_both_players_and_opens_the_career_car_select() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    // Player 2 on the arrow block seats player 1 on the keyboard, and the other way round for the keyboard.
    for (player_two, player_one) in
        [(PlayerDevice::KeyboardArrows, PlayerDevice::Keyboard), (PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows)]
    {
        let Some(mut h) = co_op_press_start() else { return };
        h.wait(1.0);
        start(&mut h, player_two);
        assert_eq!(h.screens.top(), Some(screen::CAR_SELECT), "{player_two:?}: {:?}", h.commands);
        assert_eq!(
            state::with(|s| s.split.seated()),
            Some([player_one, player_two]),
            "player 1 takes the other device of the pair"
        );
    }
}

#[test]
fn co_op_second_car_starts_a_co_op_session() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _off = CoopOff;
    let Some(mut rig) = install_rig() else { return };
    set_coop_players(1);
    // The career car select sends this when player 2 accepts a car (`car_select.rs`, `accept_coop`).
    let picked = Command::CoopSecondCar { handle: 7, transmission: Transmission::Manual };
    rig.with(|fe, host, env| fe.apply(vec![picked], host, env));
    assert_eq!(coop_players(), 2, "player 2 joins the career");
    // With no profile loaded the career resumes at the safe house menu, as `resume_career` does.
    assert_eq!(rig.fe.screens.top(), Some(screen::MAIN_MENU_SUB));
}

#[test]
fn back_from_co_op_press_start_returns_to_the_career_menu() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _reset = Reset;
    let Some(mut h) = co_op_press_start() else { return };
    h.wait(1.0);
    assert_eq!(text_of(&h, 0xB244_CF71).as_deref(), Some("Player 2 press START."));
    h.press(pad::BACK);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "back to the career menu");
    assert_eq!(state::with(|s| s.split.seated()), None, "the seats are given up");
}
