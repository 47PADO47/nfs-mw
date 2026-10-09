//! Speech inside [`Audio`]: loaded on first use, updated every frame.

use kira::track::TrackBuilder;

use super::random::Rng;
use super::{Context, Files, KiraSink, Outcome, Speech};
use crate::audio::Audio;

/// The speech of an [`Audio`]: not loaded yet, failed to load, or ready; and the volume it will have.
pub struct SpeechSlot {
    state: State,
    volume: f32,
    context: Context,
    pursuit_secs: f32,
}

enum State {
    Unloaded,
    Failed(String),
    Ready(Box<Speech>),
}

impl Default for SpeechSlot {
    fn default() -> Self {
        Self { state: State::Unloaded, volume: 0.9, context: Context::default(), pursuit_secs: 0.0 }
    }
}

/// The recordings and their index, relative to the install.
const INDEX: &str = "SOUND/SPEECH/copspeech.idx";
const BIG: &str = "SOUND/SPEECH/copspeech.big";

impl Audio {
    /// Speech, loading it the first time (the events from the attribute database, the recordings from the install).
    pub(in crate::audio) fn speech(&mut self) -> Result<&mut Speech, String> {
        if matches!(self.speech.state, State::Unloaded) {
            self.speech.state = match self.load_speech() {
                Ok(speech) => State::Ready(Box::new(speech)),
                Err(e) => {
                    log::warn!("no speech: {e}");
                    State::Failed(e)
                }
            };
        }
        match &mut self.speech.state {
            State::Ready(speech) => Ok(speech),
            State::Failed(e) => Err(format!("speech could not be loaded: {e}")),
            State::Unloaded => Err("speech is not loaded".into()),
        }
    }

    fn load_speech(&mut self) -> Result<Speech, String> {
        let db = self.database()?;
        let events = nfsmw_data::speech::events(&db);
        if events.is_empty() {
            return Err("the attribute database has no speech events".into());
        }
        let tune = nfsmw_data::speech::tune(&db);
        let index = self.dir.read(INDEX).map_err(|e| format!("{INDEX}: {e}"))?;
        let big = self.dir.resolve(BIG).ok_or_else(|| format!("{BIG} is not in the install"))?;
        let files = Files::open(&index, big, Rng::from_clock())?;
        let track = match self.output.as_mut() {
            Some(out) => Some(out.manager.add_sub_track(TrackBuilder::new()).map_err(|e| e.to_string())?),
            None => None,
        };
        let mut speech = Speech::new(events, tune, Box::new(files), Box::new(KiraSink::new(track)), Rng::from_clock());
        speech.set_volume(self.speech.volume);
        Ok(speech)
    }

    /// Once per frame, with the time since the last one. Does nothing until speech has been asked for.
    pub fn update_speech(&mut self, dt: f32) {
        let State::Ready(speech) = &mut self.speech.state else { return };
        speech.set_context(self.speech.context);
        speech.dispatcher_mut().set_pursuit_secs(self.speech.pursuit_secs);
        speech.update(dt);
    }

    /// The speech volume (linear, 0 to 1). Zero refuses requests and keeps speech from loading.
    pub fn set_speech_volume(&mut self, amplitude: f32) {
        self.speech.volume = amplitude;
        if let State::Ready(speech) = &mut self.speech.state {
            speech.set_volume(amplitude);
        }
    }

    /// Ask for a speech event (by its `SpeechID`) to be said by `speaker` (1 to 9; 0 for any). This is what gameplay
    /// calls; nothing does yet, because there are no pursuits.
    #[allow(dead_code)]
    pub fn say(&mut self, event: u32, speaker: u16) -> Result<Outcome, String> {
        if self.speech.volume <= super::AUDIBLE {
            return Ok(Outcome::Muted);
        }
        Ok(self.speech()?.request(event, speaker))
    }

    /// The state of the pursuit the speech rules look at: the wanted level, the player's speed in miles per hour
    /// and how long the pursuit has lasted. Nothing sets it yet.
    #[allow(dead_code)]
    pub fn set_speech_context(&mut self, heat: i32, player_mph: f32, pursuit_secs: f32) {
        self.speech.context = Context { heat, player_speed: player_mph };
        self.speech.pursuit_secs = pursuit_secs;
    }
}
