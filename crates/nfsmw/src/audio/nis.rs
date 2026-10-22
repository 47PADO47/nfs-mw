//! The audio stream of a NIS scene (`SOUND/STREAMS/NISAudio.big`). Spec: `docs/specs/nis-events.md` section 7.
//!
//! The NIS player asks for audio through [`NisAudioRequest`], which its scene returns from `Scene::nis_audio`: the
//! scene's name while the scene plays, and `None` when it has none. The audio plugin starts the scene's take when the
//! request changes to a scene, and stops the voice when the request changes again. Each frame the plugin reports
//! whether the voice still plays through `Scene::note_nis_audio`. A take that ends is not started again until the
//! request changes.
use super::radio::feeder::FileSource;
use super::{Audio, Group, pcm};
use ea_audio::speech::{Sentences, SpeechIndex};
use game_install::GameDir;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use nfsmw_data::sound::nis_take;
use std::time::Duration;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const EVENTS: &str = "SOUND/STREAMS/NISAudio.evt";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";

/// What the NIS player wants playing. `scene` is the NIS scene's name while the scene plays, and `None` otherwise. A
/// scene outside the NIS table (an FMV, a replay or a moment) has no stream and plays nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NisAudioRequest {
    pub scene: Option<String>,
}

/// The scene the voice was last asked for. A change stops the voice; a `Some` change also starts that scene.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Requested(Option<String>);

impl Requested {
    /// Record `wanted`. Returns true when it differs from the last request, which means the voice must follow it.
    pub fn set(&mut self, wanted: Option<&str>) -> bool {
        if self.0.as_deref() == wanted {
            return false;
        }
        self.0 = wanted.map(str::to_owned);
        true
    }
}

/// The voice of the NIS audio and the request it follows.
#[derive(Default)]
pub(super) struct NisAudio {
    requested: Requested,
    voice: Option<StaticSoundHandle>,
}

impl Drop for NisAudio {
    fn drop(&mut self) {
        self.stop();
    }
}

impl NisAudio {
    /// Whether the voice still plays. False when no scene is playing, or when its take has ended.
    pub fn playing(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.state() != PlaybackState::Stopped)
    }

    /// Stop the voice at once.
    pub fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Tween::default() });
        }
    }

    /// Decode the take of `scene` from the install and hand it to `play`, which plays it in the audio system. Returns
    /// whether a take started. A scene with no take, or a take that fails to load, starts nothing and says why in the
    /// log.
    pub fn start(
        &mut self,
        dir: &GameDir,
        scene: &str,
        play: impl FnOnce(StaticSoundData) -> Result<StaticSoundHandle, String>,
    ) -> bool {
        self.stop();
        let sound = match load(dir, scene) {
            Ok(Some(sound)) => sound,
            Ok(None) => {
                log::info!("NIS audio: {scene} has no stream");
                return false;
            }
            Err(error) => {
                log::warn!("NIS audio unavailable for {scene}: {error}");
                return false;
            }
        };
        match play(sound) {
            Ok(voice) => {
                self.voice = Some(voice);
                true
            }
            Err(error) => {
                log::warn!("NIS audio cannot play {scene}: {error}");
                false
            }
        }
    }
}

/// Read the take of `scene` from the install. `Ok(None)` when the scene has no stream.
fn load(dir: &GameDir, scene: &str) -> Result<Option<StaticSoundData>, String> {
    let index = dir.read(INDEX).map_err(|e| e.to_string())?;
    let events = dir.read(EVENTS).map_err(|e| e.to_string())?;
    let parsed = SpeechIndex::parse(&index).map_err(|e| e.to_string())?;
    let sentences = Sentences::parse(&events).map_err(|e| e.to_string())?;
    let Some(take) = nis_take(&parsed, &sentences, scene) else { return Ok(None) };
    let path = dir.resolve(STREAM).ok_or("NISAudio.big missing")?;
    let file = FileSource::open(path).map_err(|e| e.to_string())?;
    let decoded = parsed.banks()[take.bank].decode(&file, take.take).map_err(|e| e.to_string())?;
    Ok(Some(pcm::sound(&decoded)))
}

impl Audio {
    /// Follow the NIS player's request. A changed request stops the voice, and a scene starts its take.
    pub(super) fn update_nis(&mut self, wanted: Option<&str>) {
        if !self.nis.requested.set(wanted) {
            return;
        }
        self.nis.stop();
        let Some(scene) = wanted else { return };
        let dir = self.dir.clone();
        let mut nis = std::mem::take(&mut self.nis);
        nis.start(&dir, scene, |sound| self.play(Group::Sfx, sound));
        self.nis = nis;
    }

    /// Stop the NIS audio at once and forget its request, so the same scene starts again when asked for.
    pub(super) fn stop_nis(&mut self) {
        self.nis.requested.set(None);
        self.nis.stop();
    }

    /// Whether the NIS audio still plays.
    pub(super) fn nis_playing(&self) -> bool {
        self.nis.playing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_changes_only_when_its_scene_does() {
        let mut requested = Requested::default();
        assert!(!requested.set(None));
        assert!(requested.set(Some("IntroNis05")));
        assert_eq!(requested, Requested(Some("IntroNis05".into())));
        assert!(!requested.set(Some("IntroNis05")));
        assert!(requested.set(Some("EndingNis04")));
        assert!(requested.set(None));
        assert_eq!(requested, Requested(None));
        assert!(!requested.set(None));
    }

    #[test]
    fn a_new_voice_is_not_playing() {
        let nis = NisAudio::default();
        assert!(!nis.playing());
    }

    #[test]
    fn the_default_request_asks_for_nothing() {
        assert_eq!(NisAudioRequest::default().scene, None);
    }
}
