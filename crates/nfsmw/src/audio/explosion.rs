//! Authored petrol-station moment stream. Spec: `petrol-station-explosion.md`.
use super::radio::feeder::FileSource;
use super::{Audio, AudioView, Group, pcm, shaped, trace::Record, volume};
use blackbox_mixmap::{InputKey, Mixer, ObjectRef};
use ea_audio::speech::SpeechIndex;
use glam::Vec3;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use nfsmw_data::sound::{MomentSound, moment_take};
use std::time::Duration;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";
const OBJECT: ObjectRef = ObjectRef { state: 0, instance: 0, object: 6 };
const RETRIGGER_SECONDS: f32 = 2.0;
const RANGE: f32 = 120.0;

struct Recording {
    data: StaticSoundData,
    definition: MomentSound,
    bank: usize,
    take: usize,
}

#[derive(Default)]
pub(super) struct Slot {
    recording: Option<Recording>,
    mixer: Option<Mixer>,
    failed: bool,
    voice: Option<StaticSoundHandle>,
    point: Vec3,
    elapsed: f32,
}

impl Slot {
    fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Tween::default() });
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.stop();
    }
}

fn publish(mixer: &mut Mixer, point: Vec3, view: AudioView) {
    mixer.set_camera(view.camera_state);
    for (index, value) in view.distances(point).into_iter().enumerate() {
        mixer.set_input(InputKey::controller(0, 0, 2, index as u8), value);
    }
    let valid = point.is_finite() && view.car_position.is_finite() && view.camera_position.is_finite();
    mixer.set_input(InputKey::controller(0, 0, 2, 15), i32::from(valid));
}

fn levels(mixer: &Mixer, definition: MomentSound) -> Option<(f32, f32)> {
    let gain = mixer.volume(OBJECT, definition.volume_slot)?;
    let gain = (((gain.clamp(0.0, 1.0) * 32767.0) as u32) >> 8).min(127) as f32 / 127.0;
    let azimuth = match definition.positioned {
        true => mixer.azimuth(OBJECT, 0)?,
        false => 0,
    };
    Some((gain, shaped::pan(azimuth)))
}

impl Audio {
    /// Warm the actual recording once, before the first impact needs it.
    pub(in crate::audio) fn prepare_explosion(&mut self) {
        if self.output.is_none() || self.explosion.failed || self.explosion.recording.is_some() {
            return;
        }
        let result = self.load_explosion();
        match result {
            Ok((recording, mixer)) => {
                self.explosion.recording = Some(recording);
                self.explosion.mixer = Some(mixer);
            }
            Err(error) => {
                self.explosion.failed = true;
                self.trace.record(Record {
                    event: "module_unavailable",
                    class: "PETROL_EXPLOSION",
                    bank: STREAM,
                    detail: &error,
                    ..Default::default()
                });
                log::warn!("petrol explosion recording unavailable: {error}");
            }
        }
    }

    fn load_explosion(&mut self) -> Result<(Recording, Mixer), String> {
        let db = self.database()?;
        let definition = MomentSound::petrol_explosion(&db).ok_or("petrol explosion moment collection missing")?;
        let index = self.dir.read(INDEX).map_err(|e| e.to_string())?;
        let (bank, take) = moment_take(&index, definition)?;
        let parsed = SpeechIndex::parse(&index).map_err(|e| e.to_string())?;
        let path = self.dir.resolve(STREAM).ok_or("NISAudio.big missing")?;
        let file = FileSource::open(path).map_err(|e| e.to_string())?;
        let decoded = parsed.banks()[bank].decode(&file, take).map_err(|e| e.to_string())?;
        let map = super::mixer::mix_map(&self.dir).map_err(|e| format!("{e:#}"))?;
        let mut mixer = Mixer::new(map, &[1]);
        mixer.attach(OBJECT, true);
        Ok((Recording { data: pcm::sound(&decoded), definition, bank, take }, mixer))
    }

    pub(super) fn stop_explosion(&mut self) {
        self.explosion.stop();
    }

    pub(super) fn update_explosion(&mut self, view: AudioView, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let slot = &mut self.explosion;
        slot.elapsed += dt;
        if slot.voice.as_ref().is_some_and(|voice| voice.state() == PlaybackState::Stopped)
            || slot.point.distance(view.car_position) > RANGE
        {
            slot.stop();
        }
        let Some(voice) = &mut slot.voice else { return };
        let (Some(recording), Some(mixer)) = (&slot.recording, &mut slot.mixer) else { return };
        publish(mixer, slot.point, view);
        mixer.process(dt);
        let Some((gain, pan)) = levels(mixer, recording.definition) else { return };
        let tween = Tween { duration: Duration::ZERO, ..Tween::default() };
        voice.set_volume(volume::decibels(gain), tween);
        voice.set_panning(pan, tween);
    }

    pub(super) fn play_explosion(&mut self, point: Vec3, view: AudioView) {
        if !point.is_finite() || point.distance(view.car_position) > RANGE {
            return;
        }
        self.prepare_explosion();
        let slot = &mut self.explosion;
        if slot.voice.as_ref().is_some_and(|voice| voice.state() != PlaybackState::Stopped)
            && slot.elapsed < RETRIGGER_SECONDS
        {
            return;
        }
        let (Some(recording), Some(mixer)) = (&slot.recording, &mut slot.mixer) else { return };
        publish(mixer, point, view);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 32767);
        mixer.process(1.0 / 60.0);
        mixer.set_input(InputKey::object(0, 0, 6, 5), 0);
        let Some((gain, pan)) = levels(mixer, recording.definition) else { return };
        let data = recording.data.clone().volume(volume::decibels(gain)).panning(pan);
        let selection = [recording.bank as i32, recording.take as i32, recording.definition.volume_slot as i32];
        slot.stop();
        slot.point = point;
        slot.elapsed = 0.0;
        match self.play(Group::Sfx, data) {
            Ok(voice) => {
                self.explosion.voice = Some(voice);
                self.trace.record(Record {
                    event: "sample_start",
                    class: "PETROL_EXPLOSION",
                    bank: STREAM,
                    params: &selection,
                    success: Some(true),
                    ..Default::default()
                });
            }
            Err(error) => log::warn!("petrol explosion cannot play: {error}"),
        }
    }
}

#[cfg(test)]
mod tests;
