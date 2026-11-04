//! Sequenced scenery one-shots. Spec: `scenery-effect-audio.md`.
use super::{Audio, AudioView, CarEvent, car::impacts::ImpactPlayback, mixer::collision::CollisionMixer};
use blackbox_carsound::CarInput;
use glam::Vec3;
use nfsmw_data::sound::{CollisionZone, EffectSound, EventKind, ImpactSound, StreamSweetener};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct WorldEffectAudio {
    pub effect: u32,
    pub zone: CollisionZone,
    pub position: Vec3,
    pub magnitude: f32,
    /// A sequence's `EMomentStrm`: stream this `aud_moment_strm` collection at `position` instead of an effect.
    pub moment: Option<u32>,
}

#[derive(Default)]
pub(super) struct Slot {
    definitions: HashMap<(u32, CollisionZone), Option<EffectSound>>,
    playback: ImpactPlayback,
    pub(in crate::audio) sweetener: sweetener::Voice,
}

impl Slot {
    fn start(&mut self, audio: &mut Audio, request: WorldEffectAudio, view: AudioView) {
        if !request.position.is_finite() {
            return;
        }
        if let Some(moment) = request.moment {
            // `unpause` names no recording: it releases the moment queued at the breaker marker within reach.
            let (moment, point) = match moment {
                super::moment_stream::UNPAUSE => {
                    let Some(target) = audio.unpause_moment(request.position) else { return };
                    target
                }
                moment => (moment, request.position),
            };
            self.sweetener.play(audio, moment, point, view);
            return;
        }
        let key = (request.effect, request.zone);
        if let std::collections::hash_map::Entry::Vacant(entry) = self.definitions.entry(key) {
            let definition = audio.database().ok().and_then(|db| EffectSound::read(&db, key.0, key.1));
            entry.insert(definition);
        }
        let Some(definition) = &self.definitions[&key] else { return };
        let Some(magnitude) = definition.magnitude(request.magnitude, view.camera_position.distance(request.position))
        else {
            return;
        };
        let sound = &definition.sound;
        let count = sound.levels.iter().take_while(|level| !level.is_empty()).count();
        if count == 0 {
            return;
        }
        let level = ((magnitude * (count - 1) as f32 + 0.5) as usize).min(count - 1);
        audio.impact_counter = audio.impact_counter.wrapping_add(1);
        let index = audio.impact_counter as usize % sound.levels[level].len();
        let stitches = audio.stitches();
        let Some(stitch) = stitches.get(sound.levels[level][index] as usize) else { return };
        let mixer = super::mixer::mix_map(&audio.dir).ok().map(CollisionMixer::new);
        let event = CarEvent {
            sound_profile: 0,
            vehicle: None,
            audio_zone: Some(request.zone),
            kind: EventKind::Detached,
            surface: 0,
            magnitude,
            front: false,
            smackable: sound.has("SMOKABLE"),
            position: request.position,
        };
        let volume = ((sound.volumes[level] * 32768.0).round() as i32).clamp(0, 32767);
        // The original's collision object skips its sweetener while world data streams in.
        let sweetener = (sound.has("EVENT") && !view.weather_streaming)
            .then(|| StreamSweetener::select(&sound.sweeteners, (magnitude * 127.0) as i32))
            .flatten();
        self.playback.collision_with_mixer(audio, stitch, sound, event, mixer, volume);
        if let Some(sweetener) = sweetener {
            self.sweetener.play(audio, sweetener.moment, request.position, view);
        }
    }
}

impl Audio {
    pub(crate) fn update_world_effect_audio(&mut self, requests: Vec<WorldEffectAudio>, view: AudioView, dt: f32) {
        let mut slot = std::mem::take(&mut self.world_effects);
        for request in requests {
            slot.start(self, request, view);
        }
        slot.playback.update(self, dt, 0.0, &CarInput::default(), &view, (1.0, 0));
        slot.sweetener.update(view, dt);
        self.world_effects = slot;
    }

    /// Streams an impact's sweetener moment when its description admits one: an `EVENT` impact, or a `TWO_CAR`
    /// impact the player's car is part of (the caller's car is the player's).
    pub(super) fn play_impact_sweetener(&mut self, sound: &ImpactSound, magnitude: f32, point: Vec3, view: AudioView) {
        if (!sound.has("EVENT") && !sound.has("TWO_CAR")) || view.weather_streaming {
            return;
        }
        let Some(sweetener) = StreamSweetener::select(&sound.sweeteners, (magnitude.clamp(0.0, 1.0) * 127.0) as i32)
        else {
            return;
        };
        let mut voice = std::mem::take(&mut self.world_effects.sweetener);
        voice.play(self, sweetener.moment, point, view);
        self.world_effects.sweetener = voice;
    }

    pub(super) fn stop_world_effect_audio(&mut self) {
        self.world_effects.playback = ImpactPlayback::default();
        self.world_effects.sweetener.stop();
    }
}

pub(in crate::audio) mod sweetener;
#[cfg(test)]
mod tests;
