//! The outro of a race: the steps after the post-race answer (`docs/specs/story-flow.md` section 1.1, states outronis,
//! outro_message and outromovie) and the chain the race leads to (section 1.5). The NIS shell race (16.8.1) enters here
//! from its intro (`intro.rs`).

use std::collections::VecDeque;

use nfsmw_career::story::{OutroStep, chain_after, outro_steps};

use super::intro::{Cue, Playing, Progress, Stand, drive, story_fields};
use super::{RaceSession, Stage};
use crate::scenes::world::WorldScene;

/// Puts the race into its outro, with `won` deciding whether the chain runs after it. The outro plays on the next frame.
pub(super) fn begin(session: &mut RaceSession, won: bool) {
    session.stage = Stage::Outro { steps: queue(session), current: Playing::Idle, won };
}

/// The steps of a race's outro, in the order they run (`story-flow.md` section 1.1 outronis, outro_message, outromovie).
fn queue(session: &RaceSession) -> VecDeque<OutroStep> {
    let story = &session.origin.race.story;
    outro_steps(&story_fields(story), story.has_outro_marker).into()
}

/// What an outro step asks of the world. The outro cutscene stands where the player's car is (the finish is not a
/// marker the rules name), and its camera track is the first one.
pub(super) fn cue(step: OutroStep) -> Cue {
    match step {
        OutroStep::Nis(scene) => Cue::Nis { scene, track: 0, at: Stand::PlayerCar },
        OutroStep::Message(id) => Cue::Message(id),
        OutroStep::Movie(name) => Cue::Movie(name),
    }
}

/// The race that runs after this one, by its `PostRaceActivity`: only after a won career race, and only when the race
/// names one (`story-flow.md` section 1.5, F-08, F-09). A lost race never reaches it.
pub(super) fn next_race(won: bool, chain: Option<&str>, career: bool) -> Option<String> {
    chain_after(won, chain).filter(|_| career)
}

impl WorldScene {
    /// One frame of the outro. When its steps are over the race ends (the caller ends it) and the chain is queued.
    pub(super) fn race_outro(&mut self, session: &mut RaceSession, dt: f32) -> bool {
        let Stage::Outro { steps, current, won } = &mut session.stage else { return true };
        let won = *won;
        let progress = drive(self, current, dt, &mut || steps.pop_front().map(cue));
        if progress == Progress::Running {
            return true;
        }
        if let Some(name) = next_race(won, session.chain.as_deref(), session.options.career) {
            self.queued_race = Some((name, session.options));
        }
        false
    }
}
