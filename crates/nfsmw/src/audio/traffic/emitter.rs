//! One stable moving traffic source, including its horn and authored whoosh pieces.
use super::{
    ENGINE_BANK, HORN_BANK, TrafficEmitter, WHOOSH_BANK,
    mixer::{Levels, TrafficMixer},
    selection,
};
use crate::audio::{
    Audio, AudioView, Group,
    aems::{DelayedSampler, EffectModule},
};
use blackbox_mixmap::MixMap;
use nfsmw_data::sound::{Stitch, StitchPiece};
use std::sync::Arc;

pub(super) struct Piece {
    pub module: EffectModule,
    data: StitchPiece,
    volume: i32,
    pitch: i32,
}

impl Piece {
    pub fn load(audio: &mut Audio, data: StitchPiece, random: &mut u32) -> Result<Self, String> {
        let volume = ((data.volume * 32767.0) as i32
            - (selection::random(random) * f32::from(data.random_volume)) as i32)
            .max(0);
        let pitch = (i32::from(data.pitch)
            - (selection::random(random) * f32::from(data.random_pitch.unsigned_abs())) as i32)
            .clamp(0, 8192);
        let module = EffectModule::load(audio, WHOOSH_BANK, "AEMS_StichWoosh", *random)?;
        Ok(Self { module, data, volume, pitch })
    }
    pub fn params(&self, levels: Levels) -> [i32; 10] {
        [
            1,
            i32::from(self.data.sample),
            ((i64::from(self.volume) * i64::from(levels.whoosh)) >> 15) as i32,
            self.pitch,
            i32::from(self.data.azimuth.wrapping_add(levels.azimuth[1] as u16)),
            i32::from(self.data.offset),
            32767,
            0,
            25000,
            0,
        ]
    }
}

pub(super) struct Horn {
    module: EffectModule,
    age: f32,
    duration: f32,
}

pub(super) struct Emitter {
    pub state: TrafficEmitter,
    mixer: TrafficMixer,
    engine: Option<EffectModule>,
    horn: Option<Horn>,
    whoosh: Vec<Piece>,
    last_horn: f32,
    last_attempt: f32,
}

fn load(audio: &mut Audio, bank: &str, class: &str, seed: u32) -> Option<EffectModule> {
    match EffectModule::load(audio, bank, class, seed) {
        Ok(module) => Some(module),
        Err(error) => {
            log::warn!("authored traffic {class} unavailable: {error}");
            audio.trace.record(crate::audio::trace::Record {
                event: "module_unavailable",
                bank,
                class,
                logical_ms: audio.trace.now_ms(),
                success: Some(false),
                detail: &error,
                ..Default::default()
            });
            None
        }
    }
}

impl Emitter {
    pub fn new(audio: &mut Audio, map: Arc<MixMap>, state: TrafficEmitter) -> Self {
        let engine = state.profile.engine.and_then(|_| load(audio, ENGINE_BANK, "FX_TRAFFIC", state.id));
        Self {
            state,
            mixer: TrafficMixer::new(map),
            engine,
            horn: None,
            whoosh: Vec::new(),
            last_horn: 0.0,
            last_attempt: 0.0,
        }
    }
    pub fn attempt_horn(
        &mut self,
        audio: &mut Audio,
        now: f32,
        view: AudioView,
        velocity: glam::Vec3,
        global_last: &mut f32,
        random: &mut u32,
    ) {
        if self.state.profile.horn.is_none()
            || self.horn.is_some()
            || now <= self.last_horn + 3.0
            || now <= self.last_attempt + 3.0
        {
            return;
        }
        let speed = velocity.dot(view.car_forward);
        if !selection::horn_in_range(view.car_position, speed, self.state.position, self.state.forward) {
            return;
        }
        if selection::random(random) >= 0.3 && now <= *global_last + 5.0 {
            self.last_attempt = now;
            return;
        }
        let duration = 1.2 + selection::random(random) * 2.8;
        self.horn(audio, duration);
        if self.horn.is_some() {
            *global_last = now;
            audio.trace.record(crate::audio::trace::Record {
                event: "traffic_horn",
                instance: u64::from(self.state.id),
                logical_ms: f64::from(now) * 1000.0,
                detail: "autonomous ENV_STATIC common horn",
                ..Default::default()
            });
        }
    }
    pub fn horn(&mut self, audio: &mut Audio, duration: f32) {
        if self.horn.is_some() || self.state.profile.horn.is_none() {
            return;
        }
        let Some(module) = load(audio, HORN_BANK, "ENV_STATIC", self.state.id) else { return };
        self.horn = Some(Horn { module, age: 0.0, duration: duration.clamp(1.2, 4.0) });
    }
    pub fn whoosh(&mut self, audio: &mut Audio, stitch: &Stitch, random: &mut u32) {
        if !self.whoosh.is_empty() {
            return;
        }
        for piece in stitch.pieces.iter().rev().filter(|p| p.family == 1) {
            match Piece::load(audio, *piece, random) {
                Ok(piece) => self.whoosh.push(piece),
                Err(error) => log::warn!("authored traffic whoosh unavailable: {error}"),
            }
        }
    }
    pub fn update(
        &mut self,
        audio: &mut Audio,
        dt: f32,
        delay: f32,
        now: f32,
        view: AudioView,
        velocities: [glam::Vec3; 2],
    ) {
        let relative = (self.state.velocity - velocities[0]).truncate().length();
        if relative <= 10.0 {
            self.whoosh.clear();
        }
        let levels = self.mixer.update(dt, &self.state, view, velocities, self.horn.is_some(), !self.whoosh.is_empty());
        if audio.trace.enabled() {
            audio.trace.record(crate::audio::trace::Record {
                event: "traffic_source_tick",
                instance: u64::from(self.state.id),
                class: "traffic_source",
                logical_ms: f64::from(now) * 1000.0,
                params: &[
                    self.state.profile.engine.map_or(-1, |v| v as i32),
                    self.state.profile.horn.map_or(-1, |v| v as i32),
                    self.state.profile.whoosh.map_or(-1, |v| v as i32),
                    (self.state.position.x * 100.0) as i32,
                    (self.state.position.y * 100.0) as i32,
                    (self.state.velocity.x * 100.0) as i32,
                    (self.state.velocity.y * 100.0) as i32,
                    levels.engine,
                    levels.horn,
                    levels.whoosh,
                    levels.azimuth[0],
                    levels.azimuth[1],
                    levels.azimuth[2],
                    levels.pitch[0],
                    levels.pitch[2],
                ],
                detail: "IDs, source cm and cm/s, engine/horn/whoosh gains, azimuths, Q12 pitches",
                ..Default::default()
            });
        }
        if let Some(module) = &mut self.engine {
            let params = engine_params(self.state, levels);
            if !module.update(&mut DelayedSampler(audio, Group::Sfx, delay), dt, &params) {
                self.engine = None;
            }
        }
        let mut end_horn = false;
        if let Some(horn) = &mut self.horn {
            horn.age += dt;
            let fade = selection::horn_fade(horn.age, horn.duration);
            let params = horn_params(self.state.profile.horn.unwrap_or(0), levels, fade);
            end_horn = !horn.module.update(&mut DelayedSampler(audio, Group::Sfx, delay), dt, &params) || fade <= 0.0;
        }
        if end_horn {
            self.horn = None;
            self.last_horn = now;
        }
        self.whoosh.retain_mut(|piece| {
            let params = piece.params(levels);
            piece.module.update(&mut DelayedSampler(audio, Group::Sfx, delay), dt, &params) && !piece.module.finished()
        });
    }
}

pub(super) fn engine_params(state: TrafficEmitter, levels: Levels) -> [i32; 9] {
    let range = (state.velocity.length() * 2.23699 * 7.314_286) as i32;
    [
        state.profile.engine.unwrap_or(0) as i32,
        levels.engine,
        levels.pitch[0] - 4096,
        levels.azimuth[0],
        range.clamp(0, 1024),
        25000,
        0,
        32767,
        0,
    ]
}

pub(super) fn horn_params(id: u32, levels: Levels, fade: f32) -> [i32; 9] {
    [id.min(50) as i32, (levels.horn as f32 * fade) as i32, levels.pitch[2], levels.azimuth[2], 0, 25000, 0, 32767, 0]
}
