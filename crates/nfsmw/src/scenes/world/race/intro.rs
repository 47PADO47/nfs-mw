//! The intro of a race: the steps between the racers standing on the grid and the countdown (`docs/specs/story-flow.md`
//! sections 1.1 and 1.2; `docs/specs/story-flow-2.md` F-01, F-03 and F-04). The rules are in
//! `nfsmw_career::story::race_life`; this module plays them: the message card, the tutorial and intro movies, and the
//! intro NIS, which starts at the race's start. [`drive`] is the stage machine both the intro and the outro run.

use std::collections::VecDeque;

use blackbox_text::StringTable;
use glam::Vec3;
use nfsmw_career::story::prologue::{flashback_message, message_card_label};
use nfsmw_career::story::{
    AfterIntroNis, IntroContext, IntroStep, StoryFields, after_intro_nis, intro_steps, nis_shell_steps,
};
use nfsmw_data::gameplay::{RaceStory, RaceType};

use super::{RaceSession, Stage};
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::scene_space::Anchor;
use crate::scenes::world::{WorldScene, space};

/// Seconds the message card stays up: the length of `SixDaysLater.fng`'s script, about 4.2 s (`story-flow-2.md` F-04).
pub(super) const MESSAGE_SECS: f32 = 4.2;
/// The language label of the message card's text: "SIX DAYS LATER..." (`story-flow-2.md` F-04).
pub(super) const MESSAGE_LABEL: u32 = 0x290C_9B44;
/// The settings bits of the tutorials (`story-flow-2.md` F-03): drag `0x40`, speed trap `0x80`, tollbooth `0x100`.
const DRAG_BIT: u32 = 0x40;
const SPEED_TRAP_BIT: u32 = 0x80;
const TOLLBOOTH_BIT: u32 = 0x100;

/// A tutorial movie and the settings bit that says it has played (`story-flow-2.md` F-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) struct Tutorial {
    pub movie: &'static str,
    pub bit: u32,
}

/// Where a NIS scene stands: at the race's start, or where the player's car stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scenes::world) enum Stand {
    RaceStart(Spawn),
    PlayerCar,
}

/// What a step asks of the world. The world starts it and reports what plays now.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::scenes::world) enum Cue {
    /// The message card of this message id.
    Message(i32),
    /// A movie, by name.
    Movie(String),
    /// A tutorial movie: its settings bit is set when it plays.
    Tutorial(Tutorial),
    /// A NIS scene with its camera track.
    Nis { scene: String, track: u32, at: Stand },
}

/// What plays now. `Idle` between steps: the next step starts as soon as the queue has one.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::scenes::world) enum Playing {
    Idle,
    /// The message card, with its text, for this many more seconds.
    Message {
        text: String,
        left: f32,
    },
    /// A movie plays: a tutorial, a story movie, or the movie a NIS takes as its argument.
    Movie,
    /// The NIS plays.
    Nis,
}

/// The world's side of a story sequence: it starts a cue, and says whether the movie or NIS that plays is over.
pub(in crate::scenes::world) trait StoryWorld {
    fn start(&mut self, cue: Cue) -> Playing;
    fn movie_over(&mut self) -> bool;
    fn nis_over(&mut self) -> bool;
}

/// Whether a sequence has more to do after this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum Progress {
    Running,
    Done,
}

/// One frame of a story sequence (the intro or the outro). What plays ends when its time runs out (a message card) or
/// when the world says it is over (a movie or a NIS). When nothing plays, the next cue starts, until the queue is empty.
/// `dt` is the frame's real time.
pub(super) fn drive<W: StoryWorld>(
    world: &mut W,
    current: &mut Playing,
    dt: f32,
    next: &mut dyn FnMut() -> Option<Cue>,
) -> Progress {
    let over = match current {
        Playing::Idle => false,
        Playing::Message { left, .. } => {
            *left -= dt;
            *left <= 0.0
        }
        Playing::Movie => world.movie_over(),
        Playing::Nis => world.nis_over(),
    };
    if over {
        *current = Playing::Idle;
    }
    while *current == Playing::Idle {
        let Some(cue) = next() else { return Progress::Done };
        *current = world.start(cue);
    }
    Progress::Running
}

/// What the intro needs of its race while it runs.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct IntroEnv {
    /// The camera track the intro NIS passes to `NIS.Play` (`IntroCameraTrack`).
    track: u32,
    /// The tutorial due before the race, when the race runs one and it has not played yet.
    tutorial: Option<Tutorial>,
    /// Where the player's car starts: the intro NIS stands there.
    spawn: Spawn,
    /// The race is the NIS shell (16.8.1): its intro leads to the outro, not to the race.
    nis_shell: bool,
}

impl IntroEnv {
    pub(super) fn of(session: &RaceSession, seen_tutorials: u32) -> Self {
        let story = &session.origin.race.story;
        Self {
            track: u32::try_from(story.intro_camera_track).unwrap_or(0),
            tutorial: tutorial_of(session.kind, session.options.career).filter(|t| seen_tutorials & t.bit == 0),
            spawn: session.plan[0].spawn,
            nis_shell: story.nis_shell,
        }
    }
}

/// The tutorial of a race kind, in a career race: drag, tollbooth and speed trap have one (`story-flow-2.md` F-03).
pub(super) fn tutorial_of(kind: RaceType, career: bool) -> Option<Tutorial> {
    if !career {
        return None;
    }
    match kind {
        RaceType::Drag => Some(Tutorial { movie: "drag_tutorial", bit: DRAG_BIT }),
        RaceType::Tollbooth => Some(Tutorial { movie: "tollbooth_tutorial", bit: TOLLBOOTH_BIT }),
        RaceType::SpeedTrap => Some(Tutorial { movie: "speedtrap_tutorial", bit: SPEED_TRAP_BIT }),
        _ => None,
    }
}

/// Pops the next intro step as a cue. A tutorial that is not due and a warp have no cue, and are passed over. A movie
/// with a NIS plays first, and the NIS then plays with no movie argument (`story-flow.md` section 1.2 rule 4).
pub(super) fn next_intro_cue(steps: &mut VecDeque<IntroStep>, env: &IntroEnv) -> Option<Cue> {
    loop {
        let step = steps.pop_front()?;
        match step {
            IntroStep::Tutorial => {
                let Some(tutorial) = env.tutorial else { continue };
                return Some(Cue::Tutorial(tutorial));
            }
            IntroStep::Message(id) => return Some(Cue::Message(id)),
            IntroStep::Movie(name) => return Some(Cue::Movie(name)),
            IntroStep::Nis { scene, movie: Some(movie) } => {
                steps.push_front(IntroStep::Nis { scene, movie: None });
                return Some(Cue::Movie(movie));
            }
            IntroStep::Nis { scene, movie: None } => {
                return Some(Cue::Nis { scene, track: env.track, at: Stand::RaceStart(env.spawn) });
            }
            IntroStep::WarpToStart => continue,
        }
    }
}

/// Where an intro that has run its steps goes: the countdown, or the outro of the NIS shell race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IntroEnd {
    Countdown,
    Outro,
}

/// The end of the intro (`story-flow.md` section 1.1 intronis): the prerace state, or the outro for the NIS shell.
pub(super) fn intro_end(nis_shell: bool) -> IntroEnd {
    match after_intro_nis(nis_shell) {
        AfterIntroNis::Prerace => IntroEnd::Countdown,
        AfterIntroNis::Outro => IntroEnd::Outro,
    }
}

/// The steps of a race's intro, in the order they run (`story-flow.md` sections 1.1 and 1.2).
pub(super) fn intro_queue(session: &RaceSession) -> VecDeque<IntroStep> {
    // A restart goes back to the pre-race state, past the intro (`docs/specs/race-wrongway.md` section 3).
    if session.restarted {
        return VecDeque::new();
    }
    let race = &session.origin.race.story;
    // The NIS shell race has no racing: its intro is its cutscene alone (`race_life::nis_shell_steps`).
    if race.nis_shell {
        return nis_shell_steps(&story_fields(race)).into();
    }
    let ctx = IntroContext {
        career: session.options.career,
        boss_race: race.boss_race,
        dday: race.dday,
        // Split screen is not in this build: no race is played in it.
        split_screen: false,
    };
    let mut fields = story_fields(race);
    // The prologue's flashbacks open on their days-ago card, which the install gives only one of its races.
    if fields.intro_message_id.is_none() && session.options.career {
        fields.intro_message_id = flashback_message(&session.event);
    }
    intro_steps(&fields, ctx).into()
}

/// The story fields of a race, as the rules read them.
pub(super) fn story_fields(story: &RaceStory) -> StoryFields {
    StoryFields {
        intro_message_id: story.intro_message_id,
        intro_movie: story.intro_movie.clone(),
        intro_nis: story.intro_nis.clone(),
        quick_race_nis: story.quick_race_nis.clone(),
        outro_nis: story.outro_nis.clone(),
        outro_message_id: story.outro_message_id,
        outro_movie: story.outro_movie.clone(),
    }
}

impl StoryWorld for WorldScene {
    fn start(&mut self, cue: Cue) -> Playing {
        match cue {
            Cue::Message(id) => {
                log::info!("race: the message {id} is shown");
                let label = message_card_label(id).unwrap_or(MESSAGE_LABEL);
                // The language table writes a line break as `^`.
                let text = self.language_text(label).replace('^', "\n");
                Playing::Message { text, left: MESSAGE_SECS }
            }
            Cue::Movie(name) => self.start_movie(&name),
            Cue::Tutorial(tutorial) => {
                self.story_movie.note_tutorial(tutorial.bit);
                self.start_movie(tutorial.movie)
            }
            Cue::Nis { scene, track, at } => self.start_race_nis(&scene, track, at),
        }
    }

    fn movie_over(&mut self) -> bool {
        self.story_movie.take_finished()
    }

    fn nis_over(&mut self) -> bool {
        !self.nis.is_playing()
    }
}

impl WorldScene {
    /// One frame of the intro. The steps run on the grid, with the car held and the racers waiting; when the last one is
    /// over the countdown starts (or, for the NIS shell, the outro).
    pub(super) fn race_intro(&mut self, session: &mut RaceSession, dt: f32) -> bool {
        let env = IntroEnv::of(session, self.story_movie.tutorials_seen());
        let Stage::Intro { steps, current } = &mut session.stage else { return true };
        let progress = drive(self, current, dt, &mut || next_intro_cue(steps, &env));
        if progress == Progress::Running {
            return true;
        }
        self.after_intro(session, env.nis_shell);
        true
    }

    /// Carries on from the end of the intro: the countdown, or the outro of the NIS shell race.
    pub(super) fn after_intro(&mut self, session: &mut RaceSession, nis_shell: bool) {
        match intro_end(nis_shell) {
            IntroEnd::Countdown => self.start_countdown(session),
            IntroEnd::Outro => self.shell_outro(session),
        }
    }

    /// Starts the countdown (or the rolling start) on the engine that stands on the grid.
    pub(super) fn start_countdown(&mut self, session: &mut RaceSession) {
        let rolling = session.origin.race.params.rolling_start;
        let Some(engine) = session.engine.as_mut() else { return };
        let started = match rolling {
            true => engine.start_now(),
            false => engine.begin_countdown(),
        };
        session.stage = Stage::Running;
        self.race_events(session, started);
    }

    /// The NIS shell race (16.8.1) has no racing: its intro is its win, counted as a career win, and the outro follows
    /// (`story-flow.md` section 1.1 intronis, the NISShell exit).
    fn shell_outro(&mut self, session: &mut RaceSession) {
        if session.options.career {
            self.career_race_finished(&session.event, 1, session.cash_value);
        }
        super::outro::begin(session, true);
    }

    /// Plays a movie for a cue: the movie's start, or a warning and nothing playing when it does not open.
    fn start_movie(&mut self, name: &str) -> Playing {
        match self.play_story_movie(name) {
            Ok(()) => Playing::Movie,
            Err(e) => {
                log::warn!("race: the movie {name} is not played: {e}");
                Playing::Idle
            }
        }
    }

    /// Plays a race NIS for a cue: at the race's start, or where the car stands. A NIS that does not start is a warning.
    fn start_race_nis(&mut self, scene: &str, track: u32, at: Stand) -> Playing {
        let started = match at {
            Stand::RaceStart(spawn) => {
                let anchor = self.race_start_anchor(spawn);
                // The car stays on its grid slot: the scene only draws it, and must not stand it where the clip ends.
                self.play_nis_at(scene, track, anchor).inspect(|()| self.nis.keep_player_in_place())
            }
            Stand::PlayerCar => self.play_nis(scene, track),
        };
        match started {
            Ok(()) => Playing::Nis,
            Err(e) => {
                log::warn!("race: the NIS {scene} is not played: {e}");
                Playing::Idle
            }
        }
    }

    /// The anchor of the race's start: the start placement, stood on the ground and facing the way the car faces.
    fn race_start_anchor(&mut self, spawn: Spawn) -> Anchor {
        let at = spawn.position;
        let (collision, _) = self.residency.world_parts();
        let ground =
            space::ground_below(collision, at.x, at.y, at.z + space::CAR_GROUND_ABOVE, at.z - 10.0).unwrap_or(at.z);
        let forward = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
        Anchor::facing(Vec3::new(at.x, at.y, ground), heading_angle(forward))
    }

    /// A game label's text in the current language; empty when no language has it.
    fn language_text(&self, label: u32) -> String {
        crate::i18n::game(label).unwrap_or_default()
    }

    /// The text of the message card that is up, if one is.
    pub(in crate::scenes::world) fn race_card_text(&self) -> Option<&str> {
        let session = self.race.as_ref()?;
        match &session.stage {
            Stage::Intro { current: Playing::Message { text, .. }, .. }
            | Stage::Outro { current: Playing::Message { text, .. }, .. } => Some(text.as_str()),
            _ => None,
        }
    }

    /// Draws the message card (`SixDaysLater.fng` stands in: the world cannot open a front-end card, so the text is drawn
    /// over a black screen).
    pub(in crate::scenes::world) fn show_race_message_card(&self, ctx: &egui::Context) {
        let Some(text) = self.race_card_text() else { return };
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("story_card")));
        painter.rect_filled(screen, 0.0, egui::Color32::BLACK);
        painter.text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(40.0),
            egui::Color32::WHITE,
        );
    }
}

#[cfg(test)]
#[path = "intro_tests.rs"]
mod tests;
