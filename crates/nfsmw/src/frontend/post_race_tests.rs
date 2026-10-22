//! The post-race screen against the install (skipped without `NFSMW_GAME_DIR`): key 2 walks the pages, the shoulder
//! buttons step the racers, and Restart asks first.

use blackbox_feng::runtime::pad;

use super::Harness;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Env};
use crate::settings::test_caps;
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

const BACK: u32 = pad::BACK;

#[test]
fn back_returns_to_the_page_before_and_from_the_standings_to_the_photo() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(SECOND);
    h.press(SECOND);
    h.wait(2.0);
    assert!(says(&h, "STAGES"), "{:?}", shown(&h));
    h.press(BACK);
    h.wait(1.0);
    assert!(says(&h, "STATISTICS"), "back goes to the page before: {:?}", shown(&h));
    h.press(BACK);
    h.wait(1.0);
    assert!(says(&h, "STANDINGS"), "{:?}", shown(&h));
    h.press(BACK);
    h.wait(1.0);
    // The sample report has a photo, so the screen was opened over it.
    assert!(h.commands.iter().any(|c| matches!(c, Command::ShowPhoto(_))), "{:?}", h.commands);
}

/// The results of a race that was not won, with the rule the world gave it.
fn lost(rule: nfsmw_career::story::LossRule) -> Option<Harness> {
    let mut h = Harness::open(screen::POST_RACE_RESULTS, Args::default())?;
    let mut report = crate::viewer::RaceReport::sample();
    report.photo = None;
    report.retry = rule;
    let logic = Box::new(crate::frontend::post_race::PostRaceLogic::new(report));
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.commands = h.screens.open_with(screen::POST_RACE_RESULTS, logic, true, &mut env);
    Some(h)
}

#[test]
fn a_lost_race_the_story_needs_can_only_be_run_again() {
    let Some(mut h) = lost(nfsmw_career::story::LossRule::Require) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "Exit asks first: {:?}", h.commands);
    assert!(!h.said(&Command::RaceAnswer(RaceAnswer::Exit)), "the race is not left");
    h.press(BACK);
    h.wait(1.0);
    assert!(h.said(&Command::RaceAnswer(RaceAnswer::Restart)), "{:?}", h.commands);
    assert!(!h.said(&Command::RaceAnswer(RaceAnswer::Exit)));
}

#[test]
fn a_lost_race_with_no_chain_offers_the_retry_and_may_be_given_up() {
    let Some(mut h) = lost(nfsmw_career::story::LossRule::Offer) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "{:?}", h.commands);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert!(h.said(&Command::RaceAnswer(RaceAnswer::Exit)), "Quit leaves: {:?}", h.commands);
}

#[test]
fn a_race_that_ends_by_script_leaves_without_asking() {
    let Some(mut h) = lost(nfsmw_career::story::LossRule::Leave) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert!(h.said(&Command::RaceAnswer(RaceAnswer::Exit)), "{:?}", h.commands);
}

#[test]
fn the_shoulders_and_key_one_work_without_the_package_answering_them() {
    let Some(mut h) = Harness::open(screen::POST_RACE_RESULTS, Args::default()) else { return };
    h.wait(1.0);
    h.press(SECOND);
    h.wait(1.0);
    h.press(pad::LTRIGGER);
    h.wait(1.0);
    assert!(says(&h, "Dingo"), "{:?}", shown(&h));
    h.press(RESTART);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "one press asks once: {:?}", h.commands);
    h.press(BACK);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::POST_RACE_RESULTS), "back closes the question");
}
