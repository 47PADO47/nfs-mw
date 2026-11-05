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
    pub countdown: bool,
    /// Distinguishes repeated one-shots of the same scene/section, such as a race restart.
    pub serial: u32,
}

/// The scene the voice was last asked for. A change stops the voice; a `Some` change also starts that scene.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Requested(Option<NisAudioRequest>);

impl Requested {
    /// Record `wanted`. Returns true when it differs from the last request, which means the voice must follow it.
    pub fn set(&mut self, wanted: Option<&NisAudioRequest>) -> bool {
        if self.0.as_ref() == wanted {
            return false;
        }
        self.0 = wanted.cloned();
        true
    }
}

/// The result of reading a take: `Ok(None)` when the scene has no stream.
type Take = Result<Option<StaticSoundData>, String>;

/// A take being decoded on its own thread; the thread leaves the result in it.
type Slot = Arc<Mutex<Option<Take>>>;

/// The take [`prefetch`] started last, with the scene it is for. A take nobody claims stays until the next prefetch.
static PREFETCHED: Mutex<Option<Prefetched>> = Mutex::new(None);

struct Prefetched {
    scene: String,
    full: Option<Slot>,
    countdown: Option<Slot>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Starts decoding the take of `scene` on a named thread, so it is ready when the scene's audio is asked for. The NIS
/// player calls it when it starts loading the scene, so the decode overlaps with the scene's own load. Asking for the
/// scene's audio later uses this take; any other scene decodes its own.
pub fn prefetch(dir: &GameDir, scene: &str) {
    *lock(&PREFETCHED) = Some(Prefetched {
        scene: scene.to_owned(),
        full: Some(spawn_take(dir, scene, false)),
        countdown: Some(spawn_take(dir, scene, true)),
    });
}

/// The prefetched take of `scene`, if the last prefetch was for it by the same name.
fn claim_prefetched(scene: &str, countdown: bool) -> Option<Slot> {
    let mut prefetched = lock(&PREFETCHED);
    let entry = prefetched.as_mut()?;
    if entry.scene != scene {
        return None;
    }
    match countdown {
        true => entry.countdown.take(),
        false => entry.full.take(),
    }
}

fn spawn_take(dir: &GameDir, scene: &str, countdown: bool) -> Slot {
    let slot: Slot = Arc::default();
    let (dir, name, out) = (dir.clone(), scene.to_owned(), slot.clone());
    let spawned = std::thread::Builder::new().name("nis-audio".into()).spawn(move || {
        let take = load(&dir, &name, countdown);
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
    fn begin(&mut self, dir: &GameDir, scene: &str, countdown: bool) {
        self.stop();
        let slot = claim_prefetched(scene, countdown).unwrap_or_else(|| spawn_take(dir, scene, countdown));
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
fn load(dir: &GameDir, scene: &str, countdown: bool) -> Take {
    let tables = tables(dir)?;
    let take = match countdown {
        true => nfsmw_data::sound::nis_countdown_take(&tables.index, &tables.sentences, scene),
        false => nis_take(&tables.index, &tables.sentences, scene),
    };
    let Some(take) = take else { return Ok(None) };
    let path = dir.resolve(STREAM).ok_or("NISAudio.big missing")?;
    let file = FileSource::open(path).map_err(|e| e.to_string())?;
    let decoded = tables.index.banks()[take.bank].decode(&file, take.take).map_err(|e| e.to_string())?;
    Ok(Some(pcm::sound(&decoded)))
}

impl Audio {
    /// Follow the NIS player's request. A changed request stops the voice, and a scene starts decoding its take, which
    /// plays on the first update after it is ready.
    pub(super) fn update_nis(&mut self, wanted: Option<&NisAudioRequest>) {
        self.play_ready_nis_take();
        if !self.nis.requested.set(wanted) {
            return;
        }
        self.trace.record(super::trace::Record {
            event: "nis_request",
            detail: wanted.and_then(|request| request.scene.as_deref()).unwrap_or("stop"),
            params: &[
                i32::from(wanted.is_some_and(|request| request.countdown)),
                wanted.map_or(0, |request| request.serial as i32),
            ],
            ..Default::default()
        });
        self.nis.stop();
        let Some(request) = wanted else { return };
        let Some(scene) = request.scene.as_deref() else { return };
        self.nis.begin(&self.dir, scene, request.countdown);
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
            Ok(voice) => {
                self.nis.voice = Some(voice);
                self.trace.note("nis_voice_start", &scene);
            }
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
#[path = "nis_delivery_tests.rs"]
mod delivery_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs installed NIS audio index, sentence table and stream"]
    fn installed_custom_race_countdown_has_its_own_audible_take() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let tables = tables(&dir).unwrap();
        let file = FileSource::open(dir.resolve(STREAM).unwrap()).unwrap();
        for scene in ["IntroNis02", "IntroNis04"] {
            let full = nis_take(&tables.index, &tables.sentences, scene).unwrap();
            let count = nfsmw_data::sound::nis_countdown_take(&tables.index, &tables.sentences, scene).unwrap();
            assert!(full.exact && count.exact);
            assert_ne!((full.bank, full.take), (count.bank, count.take));
            if scene == "IntroNis04" {
                assert_eq!((full.bank, full.take), (2, 14));
                assert_eq!((count.bank, count.take), (2, 15));
            }
            for (section, take) in [("complete", full), ("countdown", count)] {
                let pcm = tables.index.banks()[take.bank].decode(&file, take.take).unwrap();
                eprintln!(
                    "NIS_AUDIO scene={scene} section={section} bank={} take={} seconds={:.3} channels={}",
                    take.bank,
                    take.take,
                    pcm.duration_secs(),
                    pcm.channels
                );
                assert!(pcm.samples.iter().any(|sample| sample.abs_diff(0) > 100));
                assert!(pcm.duration_secs() > 0.0);
            }
        }
    }

    #[test]
    fn a_request_changes_when_its_scene_section_or_serial_changes() {
        let mut requested = Requested::default();
        let intro = NisAudioRequest { scene: Some("IntroNis05".into()), countdown: false, serial: 0 };
        let countdown = NisAudioRequest { countdown: true, ..intro.clone() };
        assert!(!requested.set(None));
        assert!(requested.set(Some(&intro)));
        assert_eq!(requested, Requested(Some(intro.clone())));
        assert!(!requested.set(Some(&intro)));
        assert!(requested.set(Some(&countdown)), "skipping starts the countdown section of the same scene");
        let restart = NisAudioRequest { serial: 1, ..countdown };
        assert!(requested.set(Some(&restart)), "the same countdown must play again after Restart");
        assert!(!requested.set(Some(&restart)), "one request still starts only once");
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
