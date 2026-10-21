//! The intro and outro steps of a race, the exit of its intro NIS, and the post-race chain
//! (`docs/specs/story-flow.md` sections 1.1, 1.2, 1.5 and 1.7; the answers to F-01, F-02, F-06, F-08, F-09, F-11 and
//! F-12 are in `docs/specs/story-flow-2.md` section 6).
//!
//! The functions return the steps in the order the race runs them. The caller plays each step and moves on.

use super::StoryFields;

/// What the race is, as the intro rules need it (`story-flow.md` sections 1.1 and 1.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntroContext {
    /// A career race. False for a quick race (`Game.IsCareerMode()` returns false).
    pub career: bool,
    /// The race's `BossRace` field is set.
    pub boss_race: bool,
    /// A D-Day race, which skips the tutorial state (section 1.1). The spec does not list the D-Day races; the caller
    /// sets this.
    pub dday: bool,
    /// Split screen: the intro NIS is not played and the player is warped to the start (section 1.1 intronis).
    pub split_screen: bool,
}

/// One step before the race starts (`story-flow.md` section 1.1, states tutorial, intro_message, intromovie and
/// intronis).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntroStep {
    /// The tutorial state: `PlayTutorial` and a wait for the movie-finished message. Every race but a D-Day race runs it.
    Tutorial,
    /// The message screen of `IntroMessageID`.
    Message(i32),
    /// The intro movie on its own (the intromovie state, section 1.2 rule 5).
    Movie(String),
    /// The intro NIS at the start marker. `movie` is the intro movie passed to the NIS player as its argument, when set
    /// (section 1.2 rules 2 to 4, F-01).
    Nis { scene: String, movie: Option<String> },
    /// Warp the player to the start trigger. The race has no NIS to play, or runs in split screen (section 1.1).
    WarpToStart,
}

/// One step after the race (`story-flow.md` section 1.1, states outronis, outro_message and outromovie).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutroStep {
    /// The outro cutscene, played when the race has an outro marker (F-12).
    Nis(String),
    /// The outro message screen.
    Message(i32),
    /// The outro movie, played last.
    Movie(String),
}

/// Where the race goes when its intro NIS completes (section 1.1 intronis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterIntroNis {
    /// The race starts: the prerace state.
    Prerace,
    /// The NIS shell race (16.8.1) has no racing; it goes to the outro.
    Outro,
}

/// The intro steps of a race, in the order they run (`story-flow.md` sections 1.1 and 1.2).
///
/// The tutorial comes first unless the race is a D-Day race, then the intro message when one is set, then the intro
/// choice. The choice decides the NIS and the movie: a quick-race boss race plays `QuickRaceNIS` and no movie; a
/// career race with no movie goes to the NIS alone; a movie with an NIS goes to the NIS with the movie as its argument;
/// a movie with no NIS plays the movie and then warps the player (F-01, F-06).
pub fn intro_steps(story: &StoryFields, ctx: IntroContext) -> Vec<IntroStep> {
    let mut steps = Vec::new();
    if !ctx.dday {
        steps.push(IntroStep::Tutorial);
    }
    steps.extend(message_step(story).map(IntroStep::Message));
    steps.extend(intro_choice(story, ctx));
    steps
}

/// The outro steps of a race, in the order they run (`story-flow.md` section 1.1 outronis, outro_message, outromovie).
///
/// `OutroNIS` plays only when the race has an outro marker. The outro message and the outro movie play when set. A
/// race with none of them has no outro steps (F-12).
pub fn outro_steps(story: &StoryFields, has_outro_marker: bool) -> Vec<OutroStep> {
    let nis = text(&story.outro_nis).filter(|_| has_outro_marker).map(|scene| OutroStep::Nis(scene.to_owned()));
    let message = story.outro_message_id.filter(|id| *id != -1).map(OutroStep::Message);
    let movie = text(&story.outro_movie).map(|movie| OutroStep::Movie(movie.to_owned()));
    nis.into_iter().chain(message).chain(movie).collect()
}

/// Where the race goes when its intro NIS completes: the prerace state, or the outro for the NIS shell
/// (`story-flow.md` section 1.1 intronis, the `NISShell` exit).
pub fn after_intro_nis(nis_shell: bool) -> AfterIntroNis {
    if nis_shell {
        return AfterIntroNis::Outro;
    }
    AfterIntroNis::Prerace
}

/// The steps of the NIS shell race (16.8.1), which has no racing: its cutscene is the outro NIS (`EndingNis03`), and the
/// race plays it at its start, as its intro. The shell has no outro marker, so the outro rule would skip it
/// (`outro_steps`). Its completion goes to the outro (`after_intro_nis`), and the race counts as won there.
pub fn nis_shell_steps(story: &StoryFields) -> Vec<IntroStep> {
    text(&story.outro_nis).map(|scene| IntroStep::Nis { scene: scene.to_owned(), movie: None }).into_iter().collect()
}

/// The post-race chain: the name of the race's `PostRaceActivity` to run next, or `None` when the chain stops
/// (`story-flow.md` section 1.5, F-08, F-09).
///
/// A lost race stops the chain: `done` is never reached after a loss. A won race runs its activity when it is set; an
/// empty name counts as not set. The caller passes only career races, since `done` runs the activity in career alone.
pub fn chain_after(won: bool, post_race_activity: Option<&str>) -> Option<String> {
    if !won {
        return None;
    }
    post_race_activity.filter(|name| !name.is_empty()).map(str::to_owned)
}

/// The intro choice of section 1.2, after the message. Each branch returns early so the rule reads top to bottom.
fn intro_choice(story: &StoryFields, ctx: IntroContext) -> Vec<IntroStep> {
    let intro_nis = text(&story.intro_nis);
    let intro_movie = text(&story.intro_movie);
    if ctx.boss_race && !ctx.career {
        // Rule 2: a quick-race boss race plays QuickRaceNIS and never the movie.
        return vec![intronis(text(&story.quick_race_nis), None, ctx)];
    }
    if intro_movie.is_none() {
        // Rule 3: no movie, so the NIS alone (or the warp when there is no NIS).
        return vec![intronis(intro_nis, None, ctx)];
    }
    if intro_nis.is_some() {
        // Rule 4: the movie goes to the NIS player as its argument.
        return vec![intronis(intro_nis, intro_movie, ctx)];
    }
    // Rule 5: a movie and no NIS. The movie plays first, then the NIS state runs with an empty name and warps.
    let movie = intro_movie.map(|movie| IntroStep::Movie(movie.to_owned()));
    movie.into_iter().chain([IntroStep::WarpToStart]).collect()
}

/// The intronis state: the NIS with its movie argument, or the warp when there is no NIS or the game is split screen.
fn intronis(scene: Option<&str>, movie: Option<&str>, ctx: IntroContext) -> IntroStep {
    if ctx.split_screen {
        return IntroStep::WarpToStart;
    }
    match scene {
        Some(scene) => IntroStep::Nis { scene: scene.to_owned(), movie: movie.map(str::to_owned) },
        None => IntroStep::WarpToStart,
    }
}

/// The message step, when `IntroMessageID` is set and not -1.
fn message_step(story: &StoryFields) -> Option<i32> {
    story.intro_message_id.filter(|id| *id != -1)
}

/// A text field read as set: `None` when it is absent or empty.
fn text(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|text| !text.is_empty())
}

#[cfg(test)]
#[path = "race_life_tests.rs"]
mod tests;
