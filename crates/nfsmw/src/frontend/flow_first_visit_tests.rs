//! The first safe house visit through the flow (`flow_first_visit.rs`): the intro movie, the rival bio that enters bin 15, the
//! bio's Accept with the rival movie, and the safe house menu with the opening seen. The rules run without the install; the
//! flow tests need `NFSMW_GAME_DIR` and skip without it.

use std::path::PathBuf;
use std::sync::Arc;

use game_install::GameDir;
use nfsmw_career::safehouse::{BlacklistStepper, FrontEndContext, StepAction};
use nfsmw_career::settings::{CareerSettings, FIRST_RIVAL_STAGE, Flags, NewCareer, PROLOGUE_STAGE};

use super::{BIO_COUNTER, bio_continue, rival_movie};
use crate::career::{Career, CareerData, CareerHandle};
use crate::frontend::flow::Stage;
use crate::frontend::flow::flow_movie::AfterMovie;
use crate::frontend::flow_movie_tests::{Rig, install_rig};
use crate::frontend::ids::screen::{MAIN_MENU_SUB, SAFE_HOUSE_RIVAL_BIO};
use crate::frontend::logic::Command;

#[test]
fn the_rival_movie_names_the_stage_in_two_digits() {
    assert_eq!(rival_movie(15), "blacklist_15");
    assert_eq!(rival_movie(3), "blacklist_03");
}

#[test]
fn the_bio_counter_plays_the_rival_movie_and_then_sets_the_opening_seen_at_stage_15() {
    let mut settings = CareerSettings::new_career();
    settings.stage = FIRST_RIVAL_STAGE;
    let mut context = FrontEndContext::CAREER;
    context.insert(FrontEndContext::BLACKLIST_SEQUENCE);
    let mut stepper = BlacklistStepper { counter: BIO_COUNTER };
    assert_eq!(stepper.advance(&mut settings, &mut context, false), StepAction::RivalMovie);
    assert!(!settings.flags.contains(Flags::OPENING_SEEN), "the flag waits until the movie is over");
    assert_eq!(stepper.advance(&mut settings, &mut context, false), StepAction::ReputationOverview);
    assert!(settings.flags.contains(Flags::OPENING_SEEN));
}

#[test]
fn the_bio_continuation_starts_the_stepper_at_its_rival_movie() {
    assert_eq!(bio_continue(), Command::BlacklistStep(BlacklistStepper { counter: BIO_COUNTER }));
}

/// A front end over the install with a career played at the prologue (stage 16), or `None` without `NFSMW_GAME_DIR`.
fn prologue_rig() -> Option<Rig> {
    let mut rig = install_rig()?;
    // The front end shares the process's career, as `plugin.rs` gives it in the game.
    rig.fe.career = CareerHandle::shared();
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    let dir = GameDir::open(PathBuf::from(root)).unwrap();
    let data = CareerData::load(&dir).expect("the career tables load");
    let career = Career::new("first", Arc::new(data), &NewCareer::default()).unwrap();
    CareerHandle::shared().set(career);
    assert_eq!(career_stage(), PROLOGUE_STAGE, "a new career starts at the prologue");
    Some(rig)
}

fn career_stage() -> u8 {
    CareerHandle::shared().with(|career| career.settings.stage).unwrap_or(0)
}

fn career_flags() -> Flags {
    CareerHandle::shared().with(|career| career.settings.flags).unwrap_or(Flags::EMPTY)
}

#[test]
fn the_first_visit_plays_the_intro_and_the_bio_enters_bin_15() {
    let Some(mut rig) = prologue_rig() else { return };
    assert!(rig.with(|fe, host, _| fe.first_visit_intro(host)), "the intro plays at stage 16");
    assert!(matches!(rig.fe.stage, Stage::Movie { .. }), "the intro movie plays: {:?}", rig.fe.stage);
    assert_eq!(career_stage(), PROLOGUE_STAGE, "the bio has not opened yet");
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert_eq!(rig.fe.screens.top(), Some(SAFE_HOUSE_RIVAL_BIO), "the rival bio opens after the intro");
    assert_eq!(career_stage(), FIRST_RIVAL_STAGE, "opening the bio enters bin 15");
    assert!(!career_flags().contains(Flags::OPENING_SEEN), "the opening is seen only after the rival movie");
    CareerHandle::shared().clear();
}

#[test]
fn the_bio_accept_plays_the_rival_movie_then_ends_at_the_safe_house_with_the_opening_seen() {
    let Some(mut rig) = prologue_rig() else { return };
    rig.with(|fe, host, _| fe.first_visit_intro(host));
    rig.with(|fe, host, env| fe.end_movie(host, env));
    rig.with(|fe, host, env| fe.apply(vec![bio_continue()], host, env));
    assert!(
        matches!(rig.fe.stage, Stage::Movie { then: AfterMovie::Command(_) }),
        "the rival movie plays and hands the sequence back: {:?}",
        rig.fe.stage
    );
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert_eq!(rig.fe.screens.top(), Some(MAIN_MENU_SUB), "the safe house menu stands in for the overview");
    assert!(rig.fe.in_safe_house, "leaving the menu autosaves and starts the world");
    assert!(career_flags().contains(Flags::OPENING_SEEN), "step 6 sets the opening seen flag");
    assert!(CareerHandle::shared().is_prelude(), "the world is gone: the menu stands before it");
    CareerHandle::shared().clear();
}

#[test]
fn a_second_visit_does_not_replay_the_intro() {
    let Some(mut rig) = prologue_rig() else { return };
    rig.with(|fe, host, _| fe.first_visit_intro(host));
    rig.with(|fe, host, env| fe.end_movie(host, env));
    rig.with(|fe, host, env| fe.apply(vec![bio_continue()], host, env));
    rig.with(|fe, host, env| fe.end_movie(host, env));
    let played = rig.with(|fe, host, _| fe.first_visit_intro(host));
    assert!(!played, "the stage is 15 now: no intro");
    assert_eq!(rig.fe.stage, Stage::Menus, "no movie plays on a second visit");
    assert_eq!(career_stage(), FIRST_RIVAL_STAGE);
    CareerHandle::shared().clear();
}
