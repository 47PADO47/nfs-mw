//! The `radio` console command.

use super::playlist::Mode;
use super::state::clock;
use super::{Context, Player, Radio};
use crate::audio::Audio;

/// Usage line for `help`.
pub const USAGE: &str = "radio [status|list|next|play <n>|on|off|shuffle|ordered]";

/// Run `radio <args>`.
pub fn command(audio: &mut Audio, args: &[&str]) -> Result<String, String> {
    let available = audio.available();
    let (radio, music) = audio.radio_and_music()?;
    let no_device = || "there is no sound device".to_owned();
    match args {
        [] | ["status"] => Ok(radio.status(available)),
        ["list"] => Ok(radio.list()),
        ["on"] => {
            radio.set_enabled(true);
            Ok("radio on: it starts with the next song".into())
        }
        ["off"] => {
            radio.set_enabled(false);
            Ok("radio off".into())
        }
        ["shuffle"] | ["ordered"] => {
            let mode = if args[0] == "shuffle" { Mode::Shuffle } else { Mode::Ordered };
            radio.set_mode(mode);
            Ok(format!(
                "radio plays {}",
                if mode == Mode::Shuffle { "shuffled, without repeats" } else { "in list order" }
            ))
        }
        ["next"] | ["skip"] => {
            let music = music.ok_or_else(no_device)?;
            radio.skip(music).map(|title| format!("radio: {title}"))
        }
        ["play", n] => {
            let music = music.ok_or_else(no_device)?;
            let n: usize = n.parse().map_err(|_| format!("{n:?} is not a song number (radio list)"))?;
            radio.play_now(n, music)
        }
        _ => Err(format!("usage: {USAGE}")),
    }
}

impl Radio {
    /// `radio play <n>`: start song `n` now.
    pub(super) fn play_now(&mut self, n: usize, music: &mut impl Player) -> Result<String, String> {
        if n >= self.songs.len() {
            return Err(format!("there are {} songs (radio list)", self.songs.len()));
        }
        self.broken = None;
        self.manual = !self.driving;
        self.enabled = true;
        self.stop();
        self.start(n, music)?;
        Ok(format!("radio: {}", self.songs[n].title))
    }

    pub(super) fn status(&self, device: bool) -> String {
        let state = match (&self.now, self.enabled) {
            (_, false) => "off".to_owned(),
            (Some(now), true) => {
                format!("playing {} ({} of {})", now.label(), clock(now.elapsed_secs), clock(now.length_secs))
            }
            (None, true) if self.driving => "on, between songs".to_owned(),
            (None, true) => "on, waiting for driving to start (radio next plays a song now)".to_owned(),
        };
        let lists = [Context::FrontEnd, Context::InGame]
            .map(|c| format!("{} {} songs", c.name(), self.lists[c.index()].len()))
            .join(", ");
        let mode = if self.mode == Mode::Shuffle { "shuffled" } else { "in list order" };
        let device = if device { "" } else { " (no sound device)" };
        let broken = self.broken.as_ref().map(|e| format!("\n  last error: {e}")).unwrap_or_default();
        let underruns = self.playing.as_ref().map(|p| p.handle.underruns()).unwrap_or(0);
        let late = if underruns > 0 { format!("\n  the decoder was late {underruns} times") } else { String::new() };
        format!("radio {state}{device}\n  lists: {lists}; {mode}{broken}{late}")
    }

    pub(super) fn list(&self) -> String {
        let on_air = self.now.as_ref().map(|n| n.song);
        self.songs
            .iter()
            .enumerate()
            .map(|(n, s)| {
                let marker = if on_air == Some(n) { '>' } else { ' ' };
                let fe = if self.lists[0].contains(n) { "FE" } else { "  " };
                let ig = if self.lists[1].contains(n) { "IG" } else { "  " };
                format!("{marker}{n:2} {fe} {ig} {} - {}", s.artist, s.title)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
