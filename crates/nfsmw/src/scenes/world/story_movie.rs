//! The movies the world plays over its view: a race's intro and outro movies and its tutorials (`docs/specs/story-flow.md`
//! section 1.1; `docs/specs/story-flow-2.md` F-01 to F-03). A movie plays full screen while the world stands still, with
//! its sound and subtitles. The console's `movie` plays one for a test.

use ea_audio::Pcm;
use game_install::GameDir;

use super::WorldScene;
use crate::input::{Action, ActionState};
use crate::movie::{MoviePlayback, show_subtitles};
use crate::viewer::Fullscreen;

const USAGE: &str = "usage: movie <name> | movie stop";

/// The movie the world is playing, if any, how it ended, and the tutorials seen.
#[derive(Default)]
pub struct StoryMovie {
    playing: Option<MoviePlayback>,
    /// The movie ended or was stopped, and the race that waits for it has not taken that yet.
    ended: bool,
    /// The tutorial bits already set: a tutorial plays once (`story-flow-2.md` F-03). Kept for the run, not the save.
    tutorials: u32,
}

impl StoryMovie {
    /// Opens the movie `name` in `dir` (a name or part of one) and plays it from its start. Refused while one plays.
    pub fn play(&mut self, dir: &GameDir, name: &str) -> Result<(), String> {
        if self.playing.is_some() {
            return Err("a movie is playing; stop it first".into());
        }
        let movie = MoviePlayback::open(dir, name, 0.0).map_err(|e| format!("{e:#}"))?;
        self.ended = false;
        self.playing = Some(movie);
        Ok(())
    }

    /// Whether a movie plays. The world stands still while one does.
    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    /// Plays `dt` seconds of real time. `skip` ends the movie at once. The movie is over when it has run out or was
    /// skipped; the world takes that with [`StoryMovie::take_finished`].
    pub fn update(&mut self, skip: bool, dt: f32) {
        let Some(movie) = self.playing.as_mut() else { return };
        if skip {
            movie.skip();
        }
        if !movie.finished() {
            movie.advance(dt);
        }
        if movie.finished() {
            self.playing = None;
            self.ended = true;
        }
    }

    /// Ends the movie now. Returns whether one played.
    pub fn stop(&mut self) -> bool {
        let was = self.playing.take().is_some();
        self.ended |= was;
        was
    }

    /// Whether a movie ended since this was last asked, and then only once.
    pub fn take_finished(&mut self) -> bool {
        std::mem::take(&mut self.ended)
    }

    /// The picture to show over the window this frame, if a movie plays.
    pub fn picture(&mut self) -> Option<Fullscreen> {
        self.playing.as_mut()?.picture()
    }

    /// The sound of the movie, once: the caller plays it.
    pub fn take_clip(&mut self) -> Option<Pcm> {
        self.playing.as_mut()?.take_clip()
    }

    /// The subtitle lines up now (see [`MoviePlayback::subtitle_lines`]).
    pub fn subtitle_lines(&self) -> (Option<&str>, Option<&str>) {
        self.playing.as_ref().map_or((None, None), |movie| movie.subtitle_lines())
    }

    /// The tutorial bits already set.
    pub fn tutorials_seen(&self) -> u32 {
        self.tutorials
    }

    /// Sets a tutorial's bit: it has played.
    pub fn note_tutorial(&mut self, bit: u32) {
        self.tutorials |= bit;
    }
}

impl WorldScene {
    /// Plays the movie `name` over the world. Refused while one plays.
    pub(in crate::scenes::world) fn play_story_movie(&mut self, name: &str) -> Result<(), String> {
        self.story_movie.play(&self.dir, name)
    }

    /// Stops the movie that plays. Returns whether one did.
    pub(in crate::scenes::world) fn stop_story_movie(&mut self) -> bool {
        self.story_movie.stop()
    }

    /// Reads the skip and plays the movie on real frame time. Runs before the world reads the frame's input, so the
    /// world can freeze while a movie plays.
    pub(in crate::scenes::world) fn update_story_movie(&mut self, input: &ActionState, dt: f32) {
        let skip = input.just_pressed(Action::MenuAccept) || input.just_pressed(Action::MenuStart);
        self.story_movie.update(skip, dt);
    }

    /// Draws the subtitles of the movie that plays.
    pub(in crate::scenes::world) fn show_story_movie(&self, ctx: &egui::Context) {
        show_subtitles(ctx, self.story_movie.subtitle_lines());
    }
}

/// Runs `movie ...` in the world: `movie <name>` plays a movie, `movie stop` ends it.
pub(in crate::scenes::world) fn movie_command(world: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    match args {
        ["stop"] => match world.stop_story_movie() {
            true => Ok("the movie is stopped".into()),
            false => Err("no movie is playing".into()),
        },
        [name] => {
            world.play_story_movie(name)?;
            Ok(format!("playing movie {name}"))
        }
        _ => Err(USAGE.into()),
    }
}
