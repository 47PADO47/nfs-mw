//! Original breaker moment recordings; separate from common FX_UVES and music filtering.
use super::super::radio::feeder::FileSource;
use super::super::{Audio, Group, pcm, trace::Record, volume};
use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use ea_audio::speech::SpeechIndex;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{Tween, sound::PlaybackState};
use nfsmw_data::sound::{MomentSound, moment_take};
use std::sync::Arc;
use std::time::Duration;

const INDEX: &str = "SOUND/STREAMS/NISAudio.idx";
const STREAM: &str = "SOUND/STREAMS/NISAudio.big";
const OBJECT: ObjectRef = ObjectRef { state: 0, instance: 0, object: 6 };

pub(super) fn feed_position(mixer: &mut Mixer, view: super::super::AudioView) {
    mixer.set_camera(view.camera_state);
    for (index, value) in view.distances(view.car_position).into_iter().enumerate() {
        mixer.set_input(InputKey::controller(0, 0, 2, index as u8), value);
    }
    let valid = view.car_position.is_finite() && view.camera_position.is_finite();
    mixer.set_input(InputKey::controller(0, 0, 2, 15), i32::from(valid));
}

struct Recording {
    sound: StaticSoundData,
    gain: f32,
    volume_slot: usize,
    bank: usize,
    take: usize,
}

#[derive(Default)]
pub(super) struct Moment {
    active: bool,
    recordings: [Option<Recording>; 2],
    failed: [bool; 2],
    voice: Option<StaticSoundHandle>,
    selected: usize,
}

impl Drop for Moment {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Moment {
    fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            voice.stop(Tween { duration: Duration::ZERO, ..Tween::default() });
        }
    }
}

impl Audio {
    /// Decode both mode branches during initial car setup, before activation can interrupt driving.
    pub(in crate::audio) fn prepare_breaker_moments(&mut self) {
        if self.output.is_none() {
            return;
        }
        let mut moment = std::mem::take(&mut self.speedbreaker.moment);
        for interactive in [false, true] {
            self.prepare_breaker_recording(&mut moment, interactive);
        }
        self.speedbreaker.moment = moment;
    }

    fn prepare_breaker_recording(&mut self, moment: &mut Moment, interactive: bool) {
        let slot = usize::from(interactive);
        if moment.recordings[slot].is_some() || moment.failed[slot] {
            return;
        }
        match self.load_breaker_moment(interactive) {
            Ok(recording) => moment.recordings[slot] = Some(recording),
            Err(error) => {
                moment.failed[slot] = true;
                self.trace.record(Record {
                    event: "module_unavailable",
                    class: "SPEEDBREAKER_MOMENT",
                    bank: STREAM,
                    detail: &error,
                    ..Default::default()
                });
                log::warn!("Speedbreaker moment unavailable: {error}");
            }
        }
    }

    pub(super) fn update_breaker_moment(&mut self, active: bool, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let mut moment = std::mem::take(&mut self.speedbreaker.moment);
        if moment.voice.as_ref().is_some_and(|voice| voice.state() == PlaybackState::Stopped) {
            moment.voice = None;
        }
        if active == moment.active {
            if let Some(recording) = &moment.recordings[moment.selected]
                && let Some(voice) = &mut moment.voice
            {
                let gain = self.breaker_moment_gain(recording);
                voice.set_volume(volume::decibels(gain), Tween { duration: Duration::ZERO, ..Tween::default() });
            }
            self.speedbreaker.moment = moment;
            return;
        }
        moment.active = active;
        if moment.voice.is_some() {
            self.trace.record(Record {
                event: "sample_stop",
                class: "SPEEDBREAKER_MOMENT",
                bank: STREAM,
                success: Some(true),
                ..Default::default()
            });
        }
        moment.stop();
        if !active {
            self.speedbreaker.moment = moment;
            return;
        }
        let interactive = self.interactive.selected();
        let slot = usize::from(interactive);
        moment.selected = slot;
        self.prepare_breaker_recording(&mut moment, interactive);
        if let Some(recording) = &moment.recordings[slot] {
            let gain = self.breaker_moment_gain(recording);
            match self.play(Group::Sfx, recording.sound.clone().volume(volume::decibels(gain))) {
                Ok(voice) => {
                    moment.voice = Some(voice);
                    self.trace.record(Record {
                        event: "sample_start",
                        class: "SPEEDBREAKER_MOMENT",
                        bank: STREAM,
                        params: &[i32::from(interactive), recording.bank as i32, recording.take as i32],
                        success: Some(true),
                        ..Default::default()
                    });
                }
                Err(error) => log::warn!("Speedbreaker moment cannot play: {error}"),
            }
        }
        self.speedbreaker.moment = moment;
    }

    fn breaker_moment_gain(&self, recording: &Recording) -> f32 {
        let gain = self
            .speedbreaker
            .mixer
            .as_ref()
            .and_then(|mixer| mixer.volume(OBJECT, recording.volume_slot))
            .unwrap_or(recording.gain);
        (((gain.clamp(0.0, 1.0) * 32767.0) as u32) >> 8).min(127) as f32 / 127.0
    }

    fn load_breaker_moment(&mut self, interactive: bool) -> Result<Recording, String> {
        let db = self.database()?;
        let definition = MomentSound::speedbreaker(&db, interactive).ok_or("Speedbreaker moment collection missing")?;
        if definition.positioned {
            return Err("positioned breaker moment requires native spatial integration".into());
        }
        let index = self.dir.read(INDEX).map_err(|e| e.to_string())?;
        let (bank, take) = moment_take(&index, definition)?;
        let parsed = SpeechIndex::parse(&index).map_err(|e| e.to_string())?;
        let path = self.dir.resolve(STREAM).ok_or("NISAudio.big missing")?;
        let file = FileSource::open(path).map_err(|e| e.to_string())?;
        let decoded = parsed.banks()[bank].decode(&file, take).map_err(|e| e.to_string())?;
        let bytes = nfsmw_data::shared::map_output(&self.dir).map_err(|e| e.to_string())?;
        let map = MixMap::parse(&bytes).map_err(|e| e.to_string())?;
        let mut mixer = Mixer::new(Arc::new(map), &[1]);
        mixer.attach(OBJECT, true);
        feed_position(&mut mixer, self.speedbreaker.view);
        mixer.process(1.0 / 60.0);
        let gain = mixer.volume(OBJECT, definition.volume_slot).ok_or("moment volume output missing")?;
        Ok(Recording { sound: pcm::sound(&decoded), gain, volume_slot: definition.volume_slot, bank, take })
    }
}

#[cfg(test)]
mod tests;
