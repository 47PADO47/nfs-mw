//! The scene-owned streams in the audio trace, and what plays now.
//!
//! Each stream a scene starts (a movie's sound, a NIS take, a moment recording) writes `stream_start` with a fresh
//! trace instance, and the same instance comes back in `stream_stop` (stopped by the host, with the reason in `detail`)
//! or `stream_end` (it had already run out). With the trace's `logical_ms` this answers which streams play at a time T,
//! who started them and what stopped them. [`Audio::active_streams`] lists what plays now, for tests and the console's
//! `audio-active`. [ours]: diagnostics only, nothing here changes what plays.
use super::Audio;
use super::trace::Record;

/// What happened to a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Start,
    /// The host stopped it while it still played.
    Stop,
    /// It had run out by itself.
    End,
}

impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "stream_start",
            Self::Stop => "stream_stop",
            Self::End => "stream_end",
        }
    }
}

/// Who owns a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// A movie's sound (the world's story movies and the front end's movies), in the music group.
    Movie,
    /// The NIS scene's take (`nis.rs`).
    Nis,
    /// The speech channel: the police radio and the phone calls.
    Speech,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Nis => "nis",
            Self::Speech => "speech",
        }
    }
}

impl Audio {
    /// Writes one stream record to the audio trace (nothing when the trace is off).
    pub(super) fn trace_stream(&self, event: Event, kind: Kind, instance: u64, detail: &str) {
        self.trace.record(Record {
            event: event.name(),
            instance,
            class: kind.name(),
            group: Some(match kind {
                Kind::Movie => "Music",
                Kind::Nis | Kind::Speech => "Sfx",
            }),
            detail,
            success: Some(true),
            logical_ms: self.trace.now_ms(),
            ..Default::default()
        });
    }

    /// Stops the line that sounds and clears the speech queue; a line that was cut is traced as a speech stop.
    pub(super) fn silence_speech(&mut self, reason: &str) {
        let busy = self.speech.channel_busy();
        let Some(speech) = self.speech.loaded_mut() else { return };
        speech.silence();
        if busy {
            self.trace_stream(Event::Stop, Kind::Speech, 0, reason);
        }
    }

    /// The streams and long voices that play now, one line each (`movie`, `nis <scene>`, `moment world`, ...). Empty
    /// when nothing a scene owns plays; the menu's and the car's own voices are not listed.
    pub(crate) fn active_streams(&self) -> Vec<String> {
        use kira::sound::PlaybackState;
        let mut out = Vec::new();
        if self.music.as_ref().is_some_and(|(handle, _)| handle.state() != PlaybackState::Stopped) {
            out.push("movie".to_owned());
        }
        if self.nis.playing() {
            out.push(format!("nis {}", self.nis.scene().unwrap_or("?")));
        }
        if self.world_effects.sweetener.is_playing() {
            out.push("moment world".to_owned());
        }
        if self.explosion_playing() {
            out.push("moment petrol".to_owned());
        }
        if self.thunder_playing() {
            out.push("moment thunder".to_owned());
        }
        if self.breaker_moment_playing() {
            out.push("moment breaker".to_owned());
        }
        if self.speech.channel_busy() {
            out.push("speech".to_owned());
        }
        out
    }
}
