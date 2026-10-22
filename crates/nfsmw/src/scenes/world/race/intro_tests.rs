use std::collections::VecDeque;

use glam::Vec3;
use nfsmw_career::story::{IntroStep, OutroStep, StoryFields};
use nfsmw_data::gameplay::{RaceStory, RaceType};

use super::super::Stage;
use super::super::outro;
use super::*;
use crate::scenes::world::road::Spawn;

/// A world that starts cues with no game behind them: its movie and NIS end when the test says so.
#[derive(Default)]
struct FakeWorld {
    started: Vec<Cue>,
    movie_ends: bool,
    nis_ends: bool,
}

impl StoryWorld for FakeWorld {
    fn start(&mut self, cue: Cue) -> Playing {
        let playing = match &cue {
            Cue::Message(_) => Playing::Message { text: "SIX DAYS LATER...".to_owned(), left: MESSAGE_SECS },
            Cue::Movie(_) | Cue::Tutorial(_) => Playing::Movie,
            Cue::Nis { .. } => Playing::Nis,
        };
        self.started.push(cue);
        playing
    }

    fn movie_over(&mut self) -> bool {
        std::mem::take(&mut self.movie_ends)
    }

    fn nis_over(&mut self) -> bool {
        std::mem::take(&mut self.nis_ends)
    }
}

fn spawn() -> Spawn {
    Spawn { position: Vec3::new(1.0, 2.0, 3.0), heading: 0.0 }
}

fn env(track: u32, tutorial: Option<Tutorial>, nis_shell: bool) -> IntroEnv {
    IntroEnv { track, tutorial, spawn: spawn(), nis_shell }
}

fn nis(scene: &str, movie: Option<&str>) -> IntroStep {
    IntroStep::Nis { scene: scene.to_owned(), movie: movie.map(str::to_owned) }
}

fn at_start(scene: &str, track: u32) -> Cue {
    Cue::Nis { scene: scene.to_owned(), track, at: Stand::RaceStart(spawn()) }
}

/// One frame of an intro's steps in `world`.
fn intro_frame(
    world: &mut FakeWorld,
    current: &mut Playing,
    steps: &mut VecDeque<IntroStep>,
    env: &IntroEnv,
    dt: f32,
) -> Progress {
    drive(world, current, dt, &mut || next_intro_cue(steps, env))
}

/// One frame of an outro's steps in `world`.
fn outro_frame(world: &mut FakeWorld, current: &mut Playing, steps: &mut VecDeque<OutroStep>, dt: f32) -> Progress {
    drive(world, current, dt, &mut || steps.pop_front().map(outro::cue))
}

#[test]
fn a_message_runs_its_seconds_then_the_nis_plays_then_the_intro_is_done() {
    let mut steps = VecDeque::from([IntroStep::Message(2), nis("IntroNis10", None)]);
    let env = env(0, None, false);
    let (mut world, mut current) = (FakeWorld::default(), Playing::Idle);
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.0), Progress::Running);
    assert_eq!(world.started, [Cue::Message(2)]);
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 4.0), Progress::Running);
    assert_eq!(world.started.len(), 1, "the card is still up");
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.5), Progress::Running);
    assert_eq!(world.started.last(), Some(&at_start("IntroNis10", 0)), "the card ran out: the NIS starts");
    world.nis_ends = true;
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.0), Progress::Done);
    assert_eq!(world.started.len(), 2);
}

#[test]
fn a_movie_plays_before_the_nis_it_is_the_argument_of() {
    let mut steps = VecDeque::from([nis("IntroNis04", Some("storyfmv_pin11"))]);
    let env = env(2, None, false);
    let (mut world, mut current) = (FakeWorld::default(), Playing::Idle);
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.0), Progress::Running);
    assert_eq!(world.started, [Cue::Movie("storyfmv_pin11".to_owned())]);
    world.movie_ends = true;
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.0), Progress::Running);
    assert_eq!(world.started.last(), Some(&at_start("IntroNis04", 2)), "the NIS follows with the intro's camera track");
    world.nis_ends = true;
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env, 0.0), Progress::Done);
}

#[test]
fn a_tutorial_plays_only_when_it_is_due_and_a_warp_plays_nothing() {
    let drag = Tutorial { movie: "drag_tutorial", bit: 0x40 };
    let mut steps = VecDeque::from([IntroStep::Tutorial, IntroStep::Message(2)]);
    assert_eq!(next_intro_cue(&mut steps, &env(0, Some(drag), false)), Some(Cue::Tutorial(drag)));
    let mut steps = VecDeque::from([IntroStep::Tutorial, IntroStep::WarpToStart]);
    assert_eq!(next_intro_cue(&mut steps, &env(0, None, false)), None, "a tutorial not due and a warp play nothing");
    assert!(steps.is_empty());
}

#[test]
fn a_race_with_no_cue_ends_its_intro_at_once() {
    let mut steps = VecDeque::from([IntroStep::Tutorial, IntroStep::WarpToStart]);
    let (mut world, mut current) = (FakeWorld::default(), Playing::Idle);
    assert_eq!(intro_frame(&mut world, &mut current, &mut steps, &env(0, None, false), 0.0), Progress::Done);
    assert!(world.started.is_empty());
}

#[test]
fn the_tutorials_are_the_career_drag_tollbooth_and_speed_trap_races() {
    assert_eq!(tutorial_of(RaceType::Drag, true), Some(Tutorial { movie: "drag_tutorial", bit: 0x40 }));
    assert_eq!(tutorial_of(RaceType::Tollbooth, true), Some(Tutorial { movie: "tollbooth_tutorial", bit: 0x100 }));
    assert_eq!(tutorial_of(RaceType::SpeedTrap, true), Some(Tutorial { movie: "speedtrap_tutorial", bit: 0x80 }));
    assert_eq!(tutorial_of(RaceType::Drag, false), None, "a quick race has none");
    assert_eq!(tutorial_of(RaceType::Circuit, true), None);
}

#[test]
fn the_nis_shell_race_goes_from_its_intro_to_the_outro() {
    assert_eq!(intro_end(true), IntroEnd::Outro);
    assert_eq!(intro_end(false), IntroEnd::Countdown);
}

#[test]
fn the_outro_plays_its_nis_then_its_message_then_its_movie_then_ends() {
    let mut steps = VecDeque::from([
        OutroStep::Nis("GenericEnd".to_owned()),
        OutroStep::Message(3),
        OutroStep::Movie("storyfmv_end".to_owned()),
    ]);
    let (mut world, mut current) = (FakeWorld::default(), Playing::Idle);
    assert_eq!(outro_frame(&mut world, &mut current, &mut steps, 0.0), Progress::Running);
    assert_eq!(
        world.started,
        [Cue::Nis { scene: "GenericEnd".to_owned(), track: 0, at: Stand::PlayerCar }],
        "the outro NIS stands where the car is"
    );
    world.nis_ends = true;
    assert_eq!(outro_frame(&mut world, &mut current, &mut steps, 0.0), Progress::Running);
    assert_eq!(world.started.last(), Some(&Cue::Message(3)));
    assert_eq!(outro_frame(&mut world, &mut current, &mut steps, MESSAGE_SECS), Progress::Running);
    assert_eq!(world.started.last(), Some(&Cue::Movie("storyfmv_end".to_owned())));
    world.movie_ends = true;
    assert_eq!(outro_frame(&mut world, &mut current, &mut steps, 0.0), Progress::Done);
    assert_eq!(world.started.len(), 3);
}

#[test]
fn the_chain_runs_after_a_win_in_the_career_only() {
    assert_eq!(outro::next_race(false, Some("next"), true), None, "a loss stops the chain");
    assert_eq!(outro::next_race(true, Some("next"), false), None, "the chain is the career's");
    assert_eq!(outro::next_race(true, None, true), None, "a race that names no next race ends the chain");
    assert_eq!(outro::next_race(true, Some("next"), true), Some("next".to_owned()));
}

#[test]
fn the_intro_is_live_and_the_outro_is_not() {
    assert!(Stage::Intro { steps: VecDeque::new(), current: Playing::Idle }.is_live());
    assert!(!Stage::Outro { steps: VecDeque::new(), current: Playing::Idle, won: true }.is_live());
}

#[test]
fn the_story_fields_of_a_race_are_its_intro_and_outro_fields() {
    let race = RaceStory {
        intro_message_id: Some(2),
        intro_movie: Some("m".to_owned()),
        intro_nis: Some("n".to_owned()),
        quick_race_nis: Some("q".to_owned()),
        outro_nis: Some("o".to_owned()),
        outro_message_id: None,
        outro_movie: Some("om".to_owned()),
        ..RaceStory::default()
    };
    assert_eq!(
        story_fields(&race),
        StoryFields {
            intro_message_id: Some(2),
            intro_movie: Some("m".to_owned()),
            intro_nis: Some("n".to_owned()),
            quick_race_nis: Some("q".to_owned()),
            outro_nis: Some("o".to_owned()),
            outro_message_id: None,
            outro_movie: Some("om".to_owned()),
        }
    );
}
