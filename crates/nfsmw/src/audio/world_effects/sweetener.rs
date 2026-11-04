//! The world's moment voice: impact sweeteners (a booth breaking apart, a car crash), sequence moments and the
//! breaker moments the marker queue releases. Admission belongs to the shared moment stream. Spec:
//! `scenery-effect-audio.md` ("Stream sweeteners", "Shared moment stream").
use super::super::moment_stream::Owner;
use super::super::radio::feeder::FileSource;
use super::super::{Audio, AudioView, Group, explosion, pcm, trace::Record, volume};
use blackbox_mixmap::{InputKey, Mixer};
use ea_audio::speech::SpeechIndex;
use glam::Vec3;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use nfsmw_data::sound::{MomentSound, moment_takes};
use std::collections::HashMap;
use std::time::Duration;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";
struct Moment {
    definition: MomentSound,
    takes: Vec<StaticSoundData>,
}

#[derive(Default)]
pub(in crate::audio) struct Voice {
    /// Decoded moments by collection key; `None` remembers one that cannot play.
    moments: HashMap<u32, Option<Moment>>,
    mixer: Option<Mixer>,
    voice: Option<StaticSoundHandle>,
    current: Option<(u32, MomentSound)>,
    point: Vec3,
    plays: usize,
}

impl Voice {
    pub fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Tween::default() });
        }
        self.current = None;
    }

    pub fn is_playing(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.state() != PlaybackState::Stopped)
    }

    /// Decodes moment `key` ahead of its release.
    pub fn prepare(&mut self, audio: &mut Audio, key: u32) {
        self.moment(audio, key);
    }

    fn moment(&mut self, audio: &mut Audio, key: u32) -> Option<&Moment> {
        self.moments
            .entry(key)
            .or_insert_with(|| {
                Self::load(audio, key).inspect_err(|error| log::warn!("moment {key:08X} unavailable: {error}")).ok()
            })
            .as_ref()
    }

    fn load(audio: &mut Audio, key: u32) -> Result<Moment, String> {
        let db = audio.database()?;
        let definition = MomentSound::by_key(&db, key).ok_or_else(|| format!("moment {key:08X} missing"))?;
        let index = audio.dir.read(INDEX).map_err(|e| e.to_string())?;
        let found = moment_takes(&index, definition)?;
        if found.is_empty() {
            return Err(format!("moment {key:08X} selects no recording"));
        }
        let parsed = SpeechIndex::parse(&index).map_err(|e| e.to_string())?;
        let file =
            FileSource::open(audio.dir.resolve(STREAM).ok_or("NISAudio.big missing")?).map_err(|e| e.to_string())?;
        let takes = found
            .into_iter()
            .map(|(bank, take)| {
                parsed.banks()[bank].decode(&file, take).map(|d| pcm::sound(&d)).map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Moment { definition, takes })
    }

    fn mixer(&mut self, audio: &Audio) -> Option<&mut Mixer> {
        if self.mixer.is_none() {
            let map = super::super::mixer::mix_map(&audio.dir).ok()?;
            let mut mixer = Mixer::new(map, &[1]);
            mixer.attach(explosion::OBJECT, true);
            self.mixer = Some(mixer);
        }
        self.mixer.as_mut()
    }

    /// Starts moment `key` at `point` when the shared stream admits it (a released queued moment plays at its
    /// marker), stopping whichever voice held the stream.
    pub fn play(&mut self, audio: &mut Audio, key: u32, point: Vec3, view: AudioView) {
        if !point.is_finite() {
            return;
        }
        let Some((rank, at)) = audio.request_moment(Owner::World, key, Some(point), &view) else { return };
        let point = at.unwrap_or(point);
        let plays = self.plays;
        let Some(moment) = self.moment(audio, key) else { return };
        let (definition, data) = (moment.definition, moment.takes[plays % moment.takes.len()].clone());
        let Some(mixer) = self.mixer(audio) else { return };
        explosion::publish(mixer, point, view);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 32767);
        mixer.process(1.0 / 60.0);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 0);
        let Some((gain, pan)) = explosion::levels(mixer, definition) else { return };
        self.stop();
        match audio.play(Group::Sfx, data.volume(volume::decibels(gain)).panning(pan)) {
            Ok(voice) => {
                self.voice = Some(voice);
                self.current = Some((key, definition));
                self.point = point;
                self.plays += 1;
                audio.moment_started(Owner::World, key, rank, Some(point));
                audio.trace.record(Record {
                    event: "sample_start",
                    class: "IMPACT_SWEETENER",
                    bank: STREAM,
                    params: &[key as i32, definition.volume_slot as i32],
                    success: Some(true),
                    ..Default::default()
                });
            }
            Err(error) => log::warn!("impact sweetener cannot play: {error}"),
        }
    }

    /// Keeps the playing moment's authored gain and pan following the camera.
    pub fn update(&mut self, view: AudioView, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if self.voice.as_ref().is_some_and(|voice| voice.state() == PlaybackState::Stopped) {
            self.stop();
        }
        let (Some(voice), Some((_, definition)), Some(mixer)) = (&mut self.voice, self.current, &mut self.mixer) else {
            return;
        };
        explosion::publish(mixer, self.point, view);
        mixer.process(dt);
        let Some((gain, pan)) = explosion::levels(mixer, definition) else { return };
        let tween = Tween { duration: Duration::ZERO, ..Tween::default() };
        voice.set_volume(volume::decibels(gain), tween);
        voice.set_panning(pan, tween);
    }

    #[cfg(test)]
    pub fn playing(&self) -> Option<u32> {
        self.current.map(|(key, _)| key)
    }

    #[cfg(test)]
    pub fn plays(&self) -> usize {
        self.plays
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.stop();
    }
}
