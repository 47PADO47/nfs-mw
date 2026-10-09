//! Speech: the police dispatch's radio lines, said one at a time through their own volume.
//!
//! Gameplay asks for an event ("the suspect is heading east"); the [`Dispatcher`] holds the request until it is
//! allowed (priority, interrupts, delays, expiry, intervals, silence between lines) and then asks the [`Speaker`]
//! to say it: a take of the recording is decoded from `copspeech.big` and played on the speech track. Spec:
//! `docs/specs/speech.md`.
//!
//! What works: the queue and its rules (from the attribute database's `speech` events), the recordings of every
//! event that has a bank of its own (28 of the 133), the speech volume. What does not: the events that are
//! sentences built from several phrases (their rules are in `copspeech.evt`, not read yet), and nothing in the
//! game asks for speech, because there is no pursuit. Nobody has listened to the result.

mod commands;
mod dispatcher;
mod glue;
mod history;
mod lines;
mod plugin;
mod random;
mod rules;
mod sink;
#[cfg(test)]
mod tests;

pub use commands::command;
pub use dispatcher::{Dispatcher, Outcome, Request, Voice};
pub use glue::SpeechSlot;
pub use lines::{Files, Lines};
pub use plugin::add as add_systems;
pub use rules::Context;
pub use sink::{KiraSink, Sink};

use nfsmw_data::speech::{SpeechEvent, SpeechTune};

use random::Rng;

/// Volumes below this switch speech off (it stops loading and requests are refused).
const AUDIBLE: f32 = 0.001;

/// The dispatcher, the recordings and the speech track.
pub struct Speech {
    dispatcher: Dispatcher,
    lines: Box<dyn Lines>,
    sink: Box<dyn Sink>,
    context: Context,
    /// Why the last line could not be said, for `speech`.
    last_error: Option<String>,
    said: u32,
}

/// Plays the line a dispatcher asks for: takes it from the recordings and puts it on the sink.
pub struct Speaker<'a> {
    pub lines: &'a mut dyn Lines,
    pub sink: &'a mut dyn Sink,
    pub error: &'a mut Option<String>,
    pub said: &'a mut u32,
}

impl Voice for Speaker<'_> {
    fn busy(&self) -> bool {
        self.sink.busy()
    }

    fn play(&mut self, event: &SpeechEvent, speaker: u16, cut: bool) -> bool {
        let pcm = match self.lines.take(event.id, speaker) {
            Ok(pcm) => pcm,
            Err(e) => {
                log::debug!("speech {}: {e}", event.name);
                *self.error = Some(format!("{}: {e}", event.name));
                return false;
            }
        };
        if cut {
            self.sink.stop();
        }
        if let Err(e) = self.sink.play(&pcm) {
            *self.error = Some(format!("{}: {e}", event.name));
            return false;
        }
        *self.said += 1;
        log::debug!("speech: {} ({:.1} s)", event.name, pcm.frames() as f32 / pcm.sample_rate.max(1) as f32);
        true
    }
}

impl Speech {
    pub fn new(
        events: Vec<SpeechEvent>,
        tune: SpeechTune,
        lines: Box<dyn Lines>,
        sink: Box<dyn Sink>,
        rng: Rng,
    ) -> Self {
        Self {
            dispatcher: Dispatcher::new(events, tune, rng),
            lines,
            sink,
            context: Context::default(),
            last_error: None,
            said: 0,
        }
    }

    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    pub fn dispatcher_mut(&mut self) -> &mut Dispatcher {
        &mut self.dispatcher
    }

    pub fn lines(&self) -> &dyn Lines {
        self.lines.as_ref()
    }

    /// The speech volume (linear, 0 to 1). Zero switches speech off: requests are refused.
    pub fn set_volume(&mut self, amplitude: f32) {
        self.sink.set_volume(amplitude);
        self.dispatcher.set_enabled(amplitude > AUDIBLE);
    }

    /// The state of the game the rules look at.
    pub fn set_context(&mut self, context: Context) {
        self.context = context;
    }

    /// Ask for an event to be said.
    pub fn request(&mut self, event: u32, speaker: u16) -> Outcome {
        self.dispatcher.request(event, Request { speaker, actor: None })
    }

    /// Once per frame, with the time since the last one.
    pub fn update(&mut self, dt: f32) {
        let mut speaker = Speaker {
            lines: &mut *self.lines,
            sink: &mut *self.sink,
            error: &mut self.last_error,
            said: &mut self.said,
        };
        self.dispatcher.update(dt, self.context, &mut speaker);
    }

    /// Say `event` at once, outside the queue and its rules, cutting any line in progress. Returns the length in
    /// seconds.
    pub fn play_now(&mut self, event: u32, speaker: u16) -> Result<f32, String> {
        let pcm = self.lines.take(event, speaker)?;
        self.sink.stop();
        self.sink.play(&pcm)?;
        Ok(pcm.frames() as f32 / pcm.sample_rate.max(1) as f32)
    }

    /// Play one recording by its place in the index, outside the queue. Returns the length in seconds.
    pub fn play_take(&mut self, bank: usize, take: usize) -> Result<f32, String> {
        let pcm = self.lines.bank_take(bank, take)?;
        self.sink.stop();
        self.sink.play(&pcm)?;
        Ok(pcm.frames() as f32 / pcm.sample_rate.max(1) as f32)
    }

    /// Stop the line that is sounding and withdraw every request.
    pub fn silence(&mut self) {
        self.sink.stop();
        self.dispatcher.reset();
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn said(&self) -> u32 {
        self.said
    }
}
