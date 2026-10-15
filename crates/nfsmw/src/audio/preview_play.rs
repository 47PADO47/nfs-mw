//! The plugin's half of the sound viewer's transport (`preview`): it plays what the scene asks for and reports the
//! playhead back. It plays in the sound-effects group, and a request replaces the sound playing now.

use kira::sound::PlaybackState;
use kira::sound::static_sound::StaticSoundHandle;

use super::preview::{Preview, Request, Status};
use super::{Audio, Group};

#[derive(Default)]
pub struct PreviewPlayer {
    handle: Option<StaticSoundHandle>,
    current: Option<(String, usize)>,
    duration: f64,
    error: Option<String>,
}

impl PreviewPlayer {
    /// Carries out the requests queued on `preview` and writes this frame's status back to it.
    pub fn service(&mut self, audio: &mut Audio, preview: &Preview) {
        for request in preview.take_requests() {
            self.apply(audio, request);
        }
        preview.set_status(self.status());
    }

    /// Stops whatever this player has playing. The viewer calls it when the scene that used it goes away.
    pub fn stop(&mut self) {
        if let Some(mut handle) = self.handle.take() {
            handle.stop(kira::Tween::default());
        }
    }

    fn apply(&mut self, audio: &mut Audio, request: Request) {
        match request {
            Request::Play { bank, index } => self.play(audio, bank, index),
            Request::TogglePause => self.toggle_pause(),
            Request::Seek(position) => {
                if let Some(handle) = &mut self.handle {
                    handle.seek_to(position.clamp(0.0, self.duration));
                }
            }
        }
    }

    fn play(&mut self, audio: &mut Audio, bank: String, index: usize) {
        self.stop();
        self.duration = 0.0;
        self.current = Some((bank.clone(), index));
        self.error = None;
        let data = match audio.bank_sound(&bank, index) {
            Ok(data) => data,
            Err(e) => return self.error = Some(e),
        };
        self.duration = data.duration().as_secs_f64();
        match audio.play(Group::Sfx, data) {
            Ok(handle) => self.handle = Some(handle),
            Err(e) => self.error = Some(e),
        }
    }

    fn toggle_pause(&mut self) {
        let Some(handle) = &mut self.handle else { return };
        match handle.state() {
            PlaybackState::Paused | PlaybackState::Pausing => handle.resume(kira::Tween::default()),
            PlaybackState::Playing | PlaybackState::Resuming => handle.pause(kira::Tween::default()),
            _ => {}
        }
    }

    fn status(&self) -> Status {
        let (playing, paused, position, finished) = match &self.handle {
            Some(handle) => {
                let state = handle.state();
                (
                    matches!(state, PlaybackState::Playing | PlaybackState::Resuming),
                    matches!(state, PlaybackState::Paused | PlaybackState::Pausing),
                    handle.position(),
                    matches!(state, PlaybackState::Stopped),
                )
            }
            None => (false, false, 0.0, false),
        };
        Status {
            current: self.current.clone(),
            playing,
            paused,
            position,
            duration: self.duration,
            finished,
            error: self.error.clone(),
        }
    }
}
