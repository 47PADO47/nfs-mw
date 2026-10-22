//! The audio stream of a NIS scene (`SOUND/STREAMS/NISAudio.big`). Spec: `docs/specs/nis-events.md` section 7.
//!
//! The NIS player asks for audio through [`NisAudioRequest`], which its scene returns from `Scene::nis_audio`: the
//! scene's name while the scene plays, and `None` when it has none. The audio plugin starts the scene's take when the
//! request changes to a scene (the take decodes on a named thread and plays once it is ready), and stops the voice when
//! the request changes again. The NIS player can [`prefetch`] a scene's take while it loads the scene. Each frame the
//! plugin reports whether the voice still plays through `Scene::note_nis_audio`. A take that ends is not started again
//! until the request changes.
use super::radio::feeder::FileSource;
use super::{Audio, Group, pcm};
use ea_audio::speech::{Sentences, SpeechIndex};
use game_install::GameDir;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use nfsmw_data::sound::nis_take;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
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

/// The result of reading a take: `Ok(None)` when the scene has no stream.
type Take = Result<Option<StaticSoundData>, String>;

/// A take being decoded on its own thread; the thread leaves the result in it.
type Slot = Arc<Mutex<Option<Take>>>;

/// The take [`prefetch`] started last, with the scene it is for. A take nobody claims stays until the next prefetch.
static PREFETCHED: Mutex<Option<(String, Slot)>> = Mutex::new(None);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Starts decoding the take of `scene` on a named thread, so it is ready when the scene's audio is asked for. The NIS
/// player calls it when it starts loading the scene, so the decode overlaps with the scene's own load. Asking for the
/// scene's audio later uses this take; any other scene decodes its own.
pub fn prefetch(dir: &GameDir, scene: &str) {
    let slot = spawn_take(dir, scene);
    *lock(&PREFETCHED) = Some((scene.to_owned(), slot));
}

/// The prefetched take of `scene`, if the last prefetch was for it by the same name.
fn claim_prefetched(scene: &str) -> Option<Slot> {
    let mut prefetched = lock(&PREFETCHED);
    let (name, _) = prefetched.as_ref()?;
    if name != scene {
        return None;
    }
    prefetched.take().map(|(_, slot)| slot)
}

fn spawn_take(dir: &GameDir, scene: &str) -> Slot {
    let slot: Slot = Arc::default();
    let (dir, name, out) = (dir.clone(), scene.to_owned(), slot.clone());
    let spawned = std::thread::Builder::new().name("nis-audio".into()).spawn(move || {
        let take = load(&dir, &name);
        *lock(&out) = Some(take);
    });
    if let Err(error) = spawned {
        *lock(&slot) = Some(Err(format!("no decoding thread: {error}")));
    }
    slot
}

/// The voice of the NIS audio and the request it follows.
#[derive(Default)]
pub(super) struct NisAudio {
    requested: Requested,
    voice: Option<StaticSoundHandle>,
    /// The scene whose take is being decoded, and the take when the thread is done.
    pending: Option<(String, Slot)>,
}

impl Drop for NisAudio {
    fn drop(&mut self) {
        self.stop();
    }
}

impl NisAudio {
    /// Whether the voice still plays. False when no scene is playing, when its take has ended, or while it is decoding.
    pub fn playing(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.state() != PlaybackState::Stopped)
    }

    /// Stop the voice at once, and forget a take that is still decoding.
    pub fn stop(&mut self) {
        self.pending = None;
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Tween::default() });
        }
    }

    /// Starts reading the take of `scene` from the install: the prefetched one when there is one, else a new thread's.
    fn begin(&mut self, dir: &GameDir, scene: &str) {
        self.stop();
        let slot = claim_prefetched(scene).unwrap_or_else(|| spawn_take(dir, scene));
        self.pending = Some((scene.to_owned(), slot));
    }

    /// The take being decoded, once it is done.
    fn take_ready(&mut self) -> Option<(String, Take)> {
        let (_, slot) = self.pending.as_ref()?;
        let take = lock(slot).take()?;
        let (scene, _) = self.pending.take()?;
        Some((scene, take))
    }
}

/// The parsed speech index and events of the NIS stream. They are the same for the whole session.
struct Tables {
    index: SpeechIndex,
    sentences: Sentences,
}

/// The tables of the stream, parsed once per session (again when a mod replaces or edits a file).
fn tables(dir: &GameDir) -> Result<Arc<Tables>, String> {
    nfsmw_data::memo(dir, &[INDEX, EVENTS], || {
        let index = SpeechIndex::parse(&dir.read(INDEX)?)?;
        let sentences = Sentences::parse(&dir.read(EVENTS)?)?;
        Ok(Tables { index, sentences })
    })
    .map_err(|e| format!("{e:#}"))
}

/// Read the take of `scene` from the install. `Ok(None)` when the scene has no stream.
fn load(dir: &GameDir, scene: &str) -> Take {
    let tables = tables(dir)?;
    let Some(take) = nis_take(&tables.index, &tables.sentences, scene) else { return Ok(None) };
    let path = dir.resolve(STREAM).ok_or("NISAudio.big missing")?;
    let file = FileSource::open(path).map_err(|e| e.to_string())?;
    let decoded = tables.index.banks()[take.bank].decode(&file, take.take).map_err(|e| e.to_string())?;
    Ok(Some(pcm::sound(&decoded)))
}

impl Audio {
    /// Follow the NIS player's request. A changed request stops the voice, and a scene starts decoding its take, which
    /// plays on the first update after it is ready.
    pub(super) fn update_nis(&mut self, wanted: Option<&str>) {
        self.play_ready_nis_take();
        if !self.nis.requested.set(wanted) {
            return;
        }
        self.nis.stop();
        let Some(scene) = wanted else { return };
        self.nis.begin(&self.dir, scene);
    }

    /// Plays the take that has finished decoding, if one has. A scene with no take, or a take that fails to load,
    /// plays nothing and says why in the log.
    fn play_ready_nis_take(&mut self) {
        let Some((scene, take)) = self.nis.take_ready() else { return };
        let sound = match take {
            Ok(Some(sound)) => sound,
            Ok(None) => {
                log::info!("NIS audio: {scene} has no stream");
                return;
            }
            Err(error) => {
                log::warn!("NIS audio unavailable for {scene}: {error}");
                return;
            }
        };
        match self.play(Group::Sfx, sound) {
            Ok(voice) => self.nis.voice = Some(voice),
            Err(error) => log::warn!("NIS audio cannot play {scene}: {error}"),
        }
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
    fn a_take_is_ready_once_its_thread_has_left_it() {
        let mut nis = NisAudio::default();
        assert!(nis.take_ready().is_none());
        let slot: Slot = Arc::default();
        nis.pending = Some(("IntroNis05".into(), slot.clone()));
        assert!(nis.take_ready().is_none(), "the thread is not done");
        *lock(&slot) = Some(Ok(None));
        let (scene, take) = nis.take_ready().expect("ready");
        assert_eq!(scene, "IntroNis05");
        assert!(matches!(take, Ok(None)));
        assert!(nis.take_ready().is_none(), "a take is taken once");
    }

    #[test]
    fn stopping_forgets_a_take_that_is_still_decoding() {
        let mut nis = NisAudio::default();
        nis.pending = Some(("IntroNis05".into(), Arc::default()));
        nis.stop();
        assert!(nis.pending.is_none());
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
