//! One movie played against a clock, apart from any scene: the decoder, the picture due now, the sound and the end.

use anyhow::{Result, bail};
use ea_audio::Pcm;
use game_install::GameDir;

use super::framing::framing;
use super::player::{Movie, Step};
use crate::settings::hd_content;
use crate::viewer::Fullscreen;

/// The movie named `name` (`ealogo`, `blacklist_03`, or the full file name), by prefix and any case.
pub(super) fn find(dir: &GameDir, name: &str) -> Result<String> {
    let wanted = name.to_ascii_lowercase();
    let wanted = wanted.trim_end_matches(".vp6");
    let mut found: Vec<String> = dir
        .files_in("MOVIES")
        .into_iter()
        .filter(|f| f.to_ascii_lowercase().trim_end_matches(".vp6").starts_with(wanted))
        .collect();
    found.sort();
    match found.len() {
        0 => bail!("no movie {name:?} (try `nfsmw list-movies`)"),
        1 => Ok(found.remove(0)),
        _ => bail!("{name:?} could be {}", found.join(", ")),
    }
}

/// Seconds played, and whether the movie is over. The sound can outlast the picture, so the end waits for both.
struct Clock {
    /// Seconds played, counted from the start offset.
    elapsed: f64,
    /// How long the sound lasts.
    sound_secs: f64,
    done: bool,
}

impl Clock {
    fn new(start: f64, sound_secs: f64) -> Self {
        Self { elapsed: start, sound_secs, done: false }
    }

    fn tick(&mut self, dt: f32) {
        self.elapsed += f64::from(dt);
    }

    /// The picture has run out: the movie is over once the sound has too.
    fn picture_ended(&mut self) {
        if self.elapsed >= self.sound_secs {
            self.done = true;
        }
    }

    /// The movie is over, now: a skip, or a decoding error that cannot be played past.
    fn end(&mut self) {
        self.done = true;
    }
}

/// One movie being played. The owner moves the clock with [`MoviePlayback::advance`], shows what
/// [`MoviePlayback::picture`] hands out, and plays [`MoviePlayback::take_clip`] once.
pub struct MoviePlayback {
    name: String,
    movie: Movie,
    audio: Option<Pcm>,
    /// The latest frame, until [`MoviePlayback::picture`] hands it out.
    frame: Option<Vec<u8>>,
    /// Whether a picture has been handed out yet.
    shown: bool,
    clock: Clock,
}

impl MoviePlayback {
    /// Opens the movie `name` in `dir` (see [`find`]) with its clock at `start` seconds.
    pub fn open(dir: &GameDir, name: &str, start: f64) -> Result<Self> {
        let file = find(dir, name)?;
        let bytes = anyhow::Context::with_context(dir.read(&format!("MOVIES/{file}")), || format!("reading {file}"))?;
        let (mut movie, audio) = anyhow::Context::with_context(Movie::open(bytes), || format!("opening {file}"))?;
        log::info!(
            "{file}: {:?} pixels, {:.1} s, {}",
            movie.size(),
            movie.duration(),
            audio.as_ref().map_or("no sound", |_| "with sound")
        );
        let mut frame = None;
        if start > 0.0 && movie.step(start)? == Step::Frame {
            frame = Some(movie.rgba().to_vec());
        }
        let sound_secs = audio.as_ref().map_or(0.0, Pcm::duration_secs);
        Ok(Self { name: file, movie, audio, frame, shown: false, clock: Clock::new(start, sound_secs) })
    }

    /// The file name of the movie, such as `ealogo_english_ntsc.vp6`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Seconds played, from the start of the movie.
    #[allow(dead_code)] // no caller yet; the subtitles and the scene over the world will read it
    pub fn elapsed(&self) -> f64 {
        self.clock.elapsed
    }

    /// Moves the clock on by `dt` seconds and decodes the frame that is due.
    pub fn advance(&mut self, dt: f32) {
        self.clock.tick(dt);
        match self.movie.step(f64::from(dt)) {
            Ok(Step::Frame) => self.frame = Some(self.movie.rgba().to_vec()),
            Ok(Step::Same) => {}
            Ok(Step::Finished) => self.clock.picture_ended(),
            Err(e) => {
                log::error!("{}: {e:#}", self.name);
                self.clock.end();
            }
        }
    }

    /// The picture to show: the latest frame if it is new, and how it is framed. `None` before the first frame.
    pub fn picture(&mut self) -> Option<Fullscreen> {
        let rgba = self.frame.take();
        if rgba.is_none() && !self.shown {
            return None;
        }
        self.shown = true;
        let size = self.movie.size();
        let (view, fit) = framing(&self.name, size, hd_content::active().fmv);
        Some(Fullscreen { size, view, fit, rgba })
    }

    /// The sound of the movie, once: the caller plays it.
    pub fn take_clip(&mut self) -> Option<Pcm> {
        self.audio.take()
    }

    /// Whether the movie is over: its picture and its sound have ended, or it was skipped.
    pub fn finished(&self) -> bool {
        self.clock.done
    }

    /// Ends the movie now, whatever is left of it.
    #[allow(dead_code)] // no caller yet; the subtitles and the scene over the world will skip with it
    pub fn skip(&mut self) {
        self.clock.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_movie_ends_when_the_picture_has_run_out_and_the_sound_has_too() {
        let mut clock = Clock::new(0.0, 2.0);
        clock.tick(1.0);
        clock.picture_ended();
        assert!(!clock.done, "the sound still has a second to play");
        clock.tick(1.5);
        clock.picture_ended();
        assert!(clock.done);
    }

    #[test]
    fn a_start_offset_counts_as_played_time() {
        let mut clock = Clock::new(3.0, 2.0);
        assert_eq!(clock.elapsed, 3.0);
        clock.picture_ended();
        assert!(clock.done, "three seconds in, the two-second sound is over");
    }

    #[test]
    fn a_skip_ends_the_movie_for_good_even_with_sound_left() {
        let mut clock = Clock::new(0.0, 10.0);
        clock.end();
        clock.tick(0.1);
        clock.picture_ended();
        assert!(clock.done);
    }
}
