//! The post-race screen against the install (skipped without `NFSMW_GAME_DIR`): key 2 walks the pages, the shoulder
//! buttons step the racers, and Restart asks first.

use blackbox_feng::runtime::pad;

use super::Harness;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command};
use crate::viewer::RaceAnswer;

const SECOND: u32 = pad::BUTTON0 << 4;
const RESTART: u32 = (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5);

/// The texts of everything drawn on the top screen.
fn shown(h: &Harness) -> Vec<String> {
    let tree = h.screens.trees().pop().unwrap();
    tree.draw_order.iter().filter_map(|i| tree.nodes[*i].text.clone()).collect()
}

fn says(h: &Harness, text: &str) -> bool {
    shown(h).iter().any(|t| t.trim() == text)
}

#[test]
fn key_two_walks_standings_statistics_stages_and_back() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    assert!(says(&h, "STANDINGS"), "{:?}", shown(&h));
    assert!(says(&h, "stock") && says(&h, "2:52.51"));
    h.press(SECOND);
    h.wait(2.0);
    assert!(says(&h, "STATISTICS"), "{:?}", shown(&h));
    assert!(says(&h, "Top Speed") && says(&h, "Traffic Collisions"), "{:?}", shown(&h));
    h.press(SECOND);
    h.wait(2.0);
    assert!(says(&h, "STAGES") && says(&h, "20%") && says(&h, "1:35.19"), "{:?}", shown(&h));
    h.press(SECOND);
    h.wait(2.0);
    assert!(says(&h, "STANDINGS"));
}

#[test]
fn the_shoulder_buttons_step_through_every_racer() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(SECOND);
    h.wait(2.0);
    h.press(pad::RTRIGGER);
    h.wait(2.0);
    assert!(says(&h, "STATISTICS") && says(&h, "Sully"), "{:?}", shown(&h));
    h.press(pad::LTRIGGER);
    h.press(pad::LTRIGGER);
    h.wait(2.0);
    assert!(says(&h, "Dingo"), "it wraps round to the last racer: {:?}", shown(&h));
}

#[test]
fn restart_asks_first_and_cancel_goes_back_to_the_page() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(RESTART);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "{:?}", h.commands);
    assert!(says(&h, "OK") && says(&h, "Cancel"), "{:?}", shown(&h));
    assert!(!h.said(&Command::RaceAnswer(RaceAnswer::Restart)), "nothing runs before the answer");
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::POST_RACE_RESULTS), "Cancel is focused and closes the dialog");
    assert!(!h.said(&Command::RaceAnswer(RaceAnswer::Restart)));
}

#[test]
fn ok_restarts_the_race() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(RESTART);
    h.wait(1.0);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert!(h.said(&Command::RaceAnswer(RaceAnswer::Restart)), "{:?}", h.commands);
}

#[test]
fn exit_leaves_without_asking() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(2.0);
    assert!(h.said(&Command::RaceAnswer(RaceAnswer::Exit)), "{:?}", h.commands);
}
