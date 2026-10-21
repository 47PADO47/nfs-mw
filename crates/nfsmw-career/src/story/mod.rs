//! Story mode: the rules that shape a race's life from the outside, written from `docs/specs/story-flow.md`.
//!
//! - [`race_life`]: the intro steps before a race (the tutorial, the message, the intro movie and NIS), the outro
//!   steps after it, the exit of the intro NIS, and the post-race chain (`story-flow.md` sections 1.1, 1.2, 1.5, 1.7).
//! - [`final_race`]: the final race 1.8.1, its timed calls, door and GPS, the win and lose triggers, and the chain
//!   after the win (`story-flow-2.md` section 5).
//! - [`prologue`]: the six-race prologue chain, its resume point and the car lot after `16.2.1` (section 2.1, 2.3, 2.7).
//! - [`chase_16_2_1`]: the scripted chase of `16.2.1` as a per-frame state machine (section 2.4).
//! - [`drive_to_target`]: the drive to the first safe house, as a state machine (section 2.5).
//!
//! The rules read the race's story fields as [`StoryFields`]. This crate does not depend on `nfsmw-data`, so the game
//! crate copies the fields of its `RaceStory` into this struct. An empty text counts as not set, as the Lua does.

pub mod chase_16_2_1;
pub mod drive_to_target;
pub mod final_race;
pub mod final_resume;
pub mod prologue;
pub mod race_life;

pub use final_race::{EndGameStep, FinalRace, FinalRaceEvent, FinalRaceInput, GpsTarget, after_win};
pub use race_life::{
    AfterIntroNis, IntroContext, IntroStep, OutroStep, after_intro_nis, chain_after, intro_steps, outro_steps,
};

/// The story fields of one race that the rules read (`career-scripts-events.md` section 1.1). `None` means the field
/// is not set; an empty text counts as not set too.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryFields {
    /// `IntroMessageID`: the message screen shown first when set and not -1.
    pub intro_message_id: Option<i32>,
    /// `IntroMovie`: the movie of the intro.
    pub intro_movie: Option<String>,
    /// `IntroNIS`: the intro cutscene at the start marker.
    pub intro_nis: Option<String>,
    /// `QuickRaceNIS`: the cutscene a boss race plays when it starts from the quick race menu.
    pub quick_race_nis: Option<String>,
    /// `OutroNIS`: the cutscene after the race, played only when the race has an outro marker.
    pub outro_nis: Option<String>,
    /// `OutroMessageID`: the message screen shown after the outro cutscene, when set and not -1.
    pub outro_message_id: Option<i32>,
    /// `OutroMovie`: the movie played last, before the race is done.
    pub outro_movie: Option<String>,
}
