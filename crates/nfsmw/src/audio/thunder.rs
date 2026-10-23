//! Original weather moment recordings and scheduling. Spec: `rain-pc.md`.
use super::radio::feeder::FileSource;
use super::{Audio, AudioView, Group, pcm, volume};
use blackbox_mixmap::{InputKey, Mixer, ObjectRef};
use ea_audio::speech::SpeechIndex;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{Tween, sound::PlaybackState};
use nfsmw_data::sound::{MomentSound, moment_takes};
use std::time::Duration;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";
const OBJECT: ObjectRef = ObjectRef { state: 0, instance: 0, object: 6 };
struct Recording {
    sound: StaticSoundData,
    definition: MomentSound,
}
pub(super) struct Slot {
    recordings: [Vec<Recording>; 2],
    failed: [Option<String>; 2],
    mixer: Option<Mixer>,
    voice: Option<StaticSoundHandle>,
    elapsed: f32,
    delay: f32,
    seed: u32,
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            recordings: [Vec::new(), Vec::new()],
            failed: [None, None],
            mixer: None,
            voice: None,
            elapsed: 0.0,
            delay: 0.0,
            seed: 0x31A5_1731,
        }
    }
}
impl Slot {
    fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Default::default() });
        }
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.stop();
    }
}
impl Audio {
    pub(in crate::audio) fn prepare_thunder(&mut self) {
        if self.output.is_none() {
            return;
        }
        for close in [false, true] {
            let slot = usize::from(close);
            if !self.thunder.recordings[slot].is_empty() || self.thunder.failed[slot].is_some() {
                continue;
            }
            match self.load_thunder(close) {
                Ok((recording, mixer)) => {
                    self.thunder.recordings[slot] = recording;
                    self.thunder.mixer = Some(mixer);
                }
                Err(error) => {
                    self.thunder.failed[slot] = Some(error.clone());
                    log::warn!("stock thunder unavailable: {error}");
                }
            }
        }
    }
    fn load_thunder(&mut self, close: bool) -> Result<(Vec<Recording>, Mixer), String> {
        let db = self.database()?;
        let definition = MomentSound::thunder(&db, close).ok_or("thunder moment collection missing")?;
        let index = self.dir.read(INDEX).map_err(|e| e.to_string())?;
        let takes = moment_takes(&index, definition)?;
        if takes.is_empty() {
            return Err("no stock thunder recordings match the selectors".into());
        }
        let parsed = SpeechIndex::parse(&index).map_err(|e| e.to_string())?;
        let file = FileSource::open(self.dir.resolve(STREAM).ok_or("weather moment stream missing")?)
            .map_err(|e| e.to_string())?;
        let recordings = takes
            .into_iter()
            .map(|(bank, take)| {
                let decoded = parsed.banks()[bank].decode(&file, take).map_err(|e| e.to_string())?;
                Ok(Recording { sound: pcm::sound(&decoded), definition })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let map = super::mixer::mix_map(&self.dir).map_err(|e| e.to_string())?;
        let mut mixer = Mixer::new(map, &[1]);
        mixer.attach(OBJECT, true);
        Ok((recordings, mixer))
    }
    pub(in crate::audio) fn update_thunder(&mut self, view: Option<AudioView>, dt: f32) {
        let Some(view) = view else {
            self.thunder.stop();
            return;
        };
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.thunder.elapsed += dt;
        if self.thunder.voice.as_ref().is_some_and(|v| v.state() == PlaybackState::Stopped) {
            self.thunder.voice = None;
        }
        if view.rain_intensity <= 0.25 || view.weather_streaming || self.thunder.elapsed <= self.thunder.delay {
            return;
        }
        self.thunder.elapsed = 0.0;
        self.thunder.seed = self.thunder.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.thunder.delay = 25.0 + (self.thunder.seed % 20) as f32;
        if self.speech.channel_busy() || self.thunder.voice.is_some() {
            return;
        }
        let selected = usize::from(view.rain_intensity >= 0.5);
        let recordings = &self.thunder.recordings[selected];
        if recordings.is_empty() {
            return;
        }
        let variant = (self.thunder.seed >> 8) as usize % recordings.len();
        let (Some(recording), Some(mixer)) = (recordings.get(variant), &mut self.thunder.mixer) else {
            return;
        };
        mixer.set_camera(view.camera_state);
        for (i, value) in view.distances(view.car_position).into_iter().enumerate() {
            mixer.set_input(InputKey::controller(0, 0, 2, i as u8), value);
        }
        mixer.set_input(InputKey::controller(0, 0, 2, 15), 1);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 32767);
        mixer.process(dt);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 0);
        let gain = mixer.volume(OBJECT, recording.definition.volume_slot).unwrap_or(0.0);
        let gain = (((gain.clamp(0.0, 1.0) * 32767.0) as u32) >> 8).min(127) as f32 / 127.0;
        let sound = recording.sound.clone().volume(volume::decibels(gain));
        match self.play(Group::Sfx, sound) {
            Ok(voice) => self.thunder.voice = Some(voice),
            Err(error) => log::warn!("stock thunder cannot play: {error}"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the installed original moment streams"]
    fn installed_thunder_streams_resolve_and_produce_audio() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let mut audio = Audio::replay(dir, 48_000);
        audio.prepare_thunder();
        assert!(audio.thunder.recordings.iter().all(|r| !r.is_empty()), "{:?}", audio.thunder.failed);
        audio.update_thunder(Some(AudioView { rain_intensity: 1.0, ..Default::default() }), 1.0 / 60.0);
        assert!(audio.thunder.voice.is_some());
        let energy = audio.render_replay(48_000).iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
        assert!(energy > 0.01, "stock thunder recording must be non-silent: {energy}");
        audio.update_thunder(None, 1.0 / 60.0);
        assert!(audio.thunder.voice.is_none());
    }
}
