//! One movie played against a clock, apart from any scene: the workers that decode it, the picture due now, the sound
//! and the end.

use std::sync::mpsc::TryRecvError;

use anyhow::{Context, Result, bail};
use blackbox_movie::Timeline;
use blackbox_text::SubtitleTrack;
use ea_audio::Pcm;
use game_install::GameDir;

use super::framing::framing;
use super::player::{Message, Picture, VideoStream};
use super::sound::SoundDecoder;
use super::subtitles;
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
    if found.len() > 1 {
        found = in_language(found);
    }
    match found.len() {
        0 => bail!("no movie {name:?} (try `nfsmw list-movies`)"),
        1 => Ok(found.remove(0)),
        _ => bail!("{name:?} could be {}", found.join(", ")),
    }
}

/// Of the files of one movie in several languages (`ealogo_english_ntsc.vp6`, `ealogo_german_pal.vp6`), the one in the
/// current language, else the English one. Files of different movies are kept as they are.
fn in_language(found: Vec<String>) -> Vec<String> {
    let current = crate::i18n::current();
    let own = current.table.as_ref().and_then(|t| t.game).map(|g| g.stem.to_ascii_lowercase());
    for language in own.into_iter().chain(std::iter::once("english".to_owned())) {
        let marker = format!("_{language}_");
        let matching: Vec<String> = found.iter().filter(|f| f.to_ascii_lowercase().contains(&marker)).cloned().collect();
        if matching.len() == 1 {
            return matching;
        }
    }
    found
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

/// What the video worker has said about the file: the picture's size and the clock of its frames.
struct Opened {
    size: [u32; 2],
    duration: f64,
    timeline: Timeline,
}

/// One movie being played. The owner moves the clock with [`MoviePlayback::advance`], shows what
/// [`MoviePlayback::picture`] hands out, and plays [`MoviePlayback::take_clip`] once.
///
/// Opening a movie only starts two workers (see `player` and `sound`): one reads the file and decodes the pictures a few
/// frames ahead of the clock, the other decodes the sound. The clock holds, and there is no picture, until both are
/// ready, so the sound and the picture start together.
pub struct MoviePlayback {
    name: String,
    video: VideoStream,
    sound: SoundDecoder,
    /// Set once the video worker has read the header.
    opened: Option<Opened>,
    /// The sound worker is done (the movie may have no sound).
    sound_ready: bool,
    /// Both workers are ready: the clock runs.
    playing: bool,
    audio: Option<Pcm>,
    /// The latest frame, until [`MoviePlayback::picture`] hands it out.
    frame: Option<Vec<u8>>,
    /// Whether a picture has been handed out yet.
    shown: bool,
    clock: Clock,
    /// The movie's subtitles, when it has any to show (see [`subtitles::load`]).
    subtitles: Option<SubtitleTrack>,
}

impl MoviePlayback {
    /// Opens the movie `name` in `dir` (see [`find`]) with its clock at `start` seconds. A movie that is not in the
    /// install is an error now; the file is read and decoded on workers, so a damaged one is logged and ends the movie
    /// when the workers find out.
    pub fn open(dir: &GameDir, name: &str, start: f64) -> Result<Self> {
        let _trace = blackbox_trace::scope("Movie: open");
        let file = find(dir, name)?;
        let path = dir
            .resolve(&format!("MOVIES/{file}"))
            .with_context(|| format!("opening {file}: it is not in the install"))?
            .to_path_buf();
        let video = VideoStream::spawn(path.clone(), start)?;
        let sound = SoundDecoder::spawn(path)?;
        let subtitles = subtitles::load(dir, &file);
        Ok(Self {
            name: file,
            video,
            sound,
            opened: None,
            sound_ready: false,
            playing: false,
            audio: None,
            frame: None,
            shown: false,
            clock: Clock::new(start, 0.0),
            subtitles,
        })
    }

    /// The file name of the movie, such as `ealogo_english_ntsc.vp6`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Seconds played, from the start of the movie.
    pub fn elapsed(&self) -> f64 {
        self.clock.elapsed
    }

    /// The subtitle lines up now: line 1 and, in the tutorials, line 2. Both are `None` when nothing is shown.
    pub fn subtitle_lines(&self) -> (Option<&str>, Option<&str>) {
        subtitles::lines_at(self.subtitles.as_ref(), self.elapsed())
    }

    /// Moves the clock on by `dt` seconds and takes the frame that is due. The clock holds while the workers are still
    /// opening the movie.
    pub fn advance(&mut self, dt: f32) {
        if self.clock.done || !self.load() {
            return;
        }
        self.clock.tick(dt);
        self.present_due();
    }

    /// Takes in what the workers have finished so far. True once the movie plays; a failure ends the movie.
    fn load(&mut self) -> bool {
        if self.playing {
            return true;
        }
        if self.opened.is_none() {
            self.poll_header();
        }
        if !self.sound_ready {
            self.poll_sound();
        }
        let Some(opened) = &self.opened else { return false };
        if !self.sound_ready || self.clock.done {
            return false;
        }
        log::info!(
            "{}: {:?} pixels, {:.1} s, {}",
            self.name,
            opened.size,
            opened.duration,
            self.audio.as_ref().map_or("no sound", |_| "with sound")
        );
        self.playing = true;
        true
    }

    fn poll_header(&mut self) {
        match self.video.try_next() {
            Ok(Message::Opened(header)) => {
                let size = [u32::from(header.width), u32::from(header.height)];
                let timeline = Timeline::from_header(&header);
                self.opened = Some(Opened { size, duration: header.duration(), timeline });
            }
            Ok(Message::Failed(e)) => self.fail(&e),
            // The header comes first, so a frame here cannot be.
            Ok(Message::Frame(_)) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.fail("the video decoder stopped"),
        }
    }

    fn poll_sound(&mut self) {
        match self.sound.try_take() {
            Ok(Ok(audio)) => {
                self.clock.sound_secs = audio.as_ref().map_or(0.0, Pcm::duration_secs);
                self.audio = audio;
                self.sound_ready = true;
            }
            Ok(Err(e)) => self.fail(&e),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.fail("the sound decoder stopped"),
        }
    }

    /// The movie cannot go on: it is logged and over.
    fn fail(&mut self, reason: &str) {
        log::error!("{}: {reason}", self.name);
        self.clock.end();
    }

    /// Takes the frame that is due at the clock from the queue of the video worker.
    fn present_due(&mut self) {
        let Some(opened) = self.opened.as_mut() else { return };
        let update = opened.timeline.update(self.clock.elapsed);
        let Some(target) = update.present else {
            if update.finished {
                self.drain_video();
                self.clock.picture_ended();
            }
            return;
        };
        self.take_frames(target);
        if update.finished {
            self.drain_video();
        }
    }

    /// Takes pictures from the worker up to frame `target`. Earlier ones are dropped: they were due already. When the
    /// worker is late, the newest one it has is shown and the rest follow on later frames.
    fn take_frames(&mut self, target: u32) {
        loop {
            match self.video.try_next() {
                Ok(Message::Frame(Picture { index, rgba })) => {
                    self.show(rgba);
                    if index >= target {
                        return;
                    }
                }
                Ok(Message::Opened(_)) => {}
                Ok(Message::Failed(e)) => return self.fail(&e),
                Err(TryRecvError::Empty) => return,
                // The file had no more frames than that.
                Err(TryRecvError::Disconnected) => return self.clock.picture_ended(),
            }
        }
    }

    /// Makes `rgba` the latest frame; the one it replaces, never shown, goes back to the worker.
    fn show(&mut self, rgba: Vec<u8>) {
        let Some(unshown) = self.frame.replace(rgba) else { return };
        self.video.give_back(unshown);
    }

    /// The picture has run out: what is left in the queue is of no use, and the worker must not wait on it.
    fn drain_video(&mut self) {
        while let Ok(message) = self.video.try_next() {
            let Message::Frame(picture) = message else { continue };
            self.video.give_back(picture.rgba);
        }
    }

    /// The picture to show: the latest frame if it is new, and how it is framed. `None` before the first frame.
    pub fn picture(&mut self) -> Option<Fullscreen> {
        let size = self.opened.as_ref()?.size;
        let rgba = self.frame.take();
        if rgba.is_none() && !self.shown {
            return None;
        }
        self.shown = true;
        let (view, fit) = framing(&self.name, size, hd_content::active().fmv);
        Some(Fullscreen { size, view, fit, rgba })
    }

    /// The sound of the movie, once: the caller plays it. `None` until the sound is decoded, and for a movie without any.
    pub fn take_clip(&mut self) -> Option<Pcm> {
        self.audio.take()
    }

    /// Whether the movie is over: its picture and its sound have ended, or it was skipped.
    pub fn finished(&self) -> bool {
        self.clock.done
    }

    /// Ends the movie now, whatever is left of it.
    pub fn skip(&mut self) {
        self.clock.end();
    }

    /// Whether the workers are done opening the movie and the clock runs.
    #[cfg(test)]
    pub(super) fn loaded(&self) -> bool {
        self.playing
    }

    /// The length of the picture in seconds, once the header is read.
    #[cfg(test)]
    pub(super) fn duration(&self) -> Option<f64> {
        self.opened.as_ref().map(|opened| opened.duration)
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

    #[test]
    fn the_clock_shows_each_subtitle_line_from_its_cue_until_the_clear() {
        use blackbox_text::Cue;
        // Cues at 5.5 s and 14.5 s (the clear); the times are exact in binary, so the clock's sums are too.
        let cues = [Cue { start_tenths: 55, label: 7 }, Cue { start_tenths: 145, label: 0xC1EA }];
        let track = SubtitleTrack::new(&cues, 0xC1EA, |_| Some("line".to_owned()));
        let lines = |clock: &Clock| subtitles::lines_at(Some(&track), clock.elapsed);
        let mut clock = Clock::new(0.0, 0.0);
        assert_eq!(lines(&clock), (None, None));
        clock.tick(5.5);
        assert_eq!(lines(&clock), (Some("line"), None));
        clock.tick(9.0);
        assert_eq!(lines(&clock), (None, None), "the clear at 14.5 s");
    }
}
