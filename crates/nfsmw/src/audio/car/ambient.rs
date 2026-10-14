//! Authored road lifetimes and weather wind; spec: car-audio-road-weather.md.
mod road;
mod stitch;
#[cfg(test)]
mod tests;
mod wind;

use crate::audio::{
    Audio, Group,
    aems::{EffectModule, EffectSampler},
    mixer::ambient::AmbientLevels,
};
use blackbox_carsound::{CarInput, LoopId, SoundCommand, SoundRef};
pub use road::{RoadContext, RoadTerrain};

const ROAD_BANK: &str = "IG_GLOBAL/ROADNOISE_00_MB.abk";
const WIND_BANK: &str = "IG_GLOBAL/WIND_00_MB.abk";

#[derive(Default)]
struct Slot {
    module: Option<EffectModule>,
    retired: Vec<(EffectModule, Vec<i32>)>,
    params: Vec<i32>,
    failed: bool,
}

impl Slot {
    fn release(&mut self) {
        let Some(mut module) = self.module.take() else { return };
        module.release();
        self.retired.push((module, self.params.clone()));
    }

    fn start(&mut self, audio: &mut Audio, bank: &str, class: &str, seed: u32) {
        self.release();
        self.failed = false;
        match EffectModule::load(audio, bank, class, seed) {
            Ok(module) => self.module = Some(module),
            Err(error) => {
                log::warn!("{class} unavailable: {error}");
                self.failed = true;
            }
        }
    }

    fn ensure(&mut self, audio: &mut Audio, bank: &str, class: &str, seed: u32) {
        if self.failed || self.module.is_some() {
            return;
        }
        self.start(audio, bank, class, seed);
    }

    fn update(&mut self, audio: &mut Audio, dt: f32, params: &[i32]) {
        self.params = params.to_vec();
        if self.module.as_mut().is_some_and(|m| !m.update(&mut EffectSampler(audio, Group::Sfx), dt, params)) {
            self.module = None;
            self.failed = true;
        }
        if self.module.as_ref().is_some_and(EffectModule::finished) {
            self.module = None;
        }
        self.retired.retain_mut(|(module, params)| {
            module.update(&mut EffectSampler(audio, Group::Sfx), dt, params) && !module.finished()
        });
    }
}

#[derive(Default)]
pub(super) struct AmbientPlayback {
    wind: Slot,
    weather: Slot,
    road: [Slot; 2],
    wet: [Slot; 2],
    transition: [Slot; 2],
    stitches: [stitch::StitchRoad; 2],
    ids: [Option<u8>; 2],
    road_params: [[i32; 11]; 2],
    generated: [i32; 2],
    secondary: [bool; 2],
    history: road::History,
}

impl AmbientPlayback {
    pub fn owns_loop(&self, id: LoopId) -> bool {
        match id {
            LoopId::Wind => self.wind.module.is_some() && !self.wind.failed,
            LoopId::Road(side) if side <= 1 => {
                let side = usize::from(side);
                if self.ids[side] == Some(8) {
                    return self.stitches[side].owns_loop();
                }
                self.ids[side].is_some() && self.road[side].module.is_some() && !self.road[side].failed
            }
            _ => false,
        }
    }
    #[cfg(test)]
    pub fn available(&self) -> bool {
        !self.wind.failed && self.wind.module.is_some() && self.road.iter().all(|s| !s.failed && s.module.is_some())
    }

    #[cfg(test)]
    pub fn update(
        &mut self,
        audio: &mut Audio,
        input: &CarInput,
        commands: &mut Vec<SoundCommand>,
        levels: AmbientLevels,
        dt: f32,
    ) {
        self.update_with_context(audio, input, commands, levels, dt, None);
    }

    pub fn update_with_context(
        &mut self,
        audio: &mut Audio,
        input: &CarInput,
        commands: &mut Vec<SoundCommand>,
        levels: AmbientLevels,
        dt: f32,
        context: Option<&RoadContext>,
    ) {
        self.wind.ensure(audio, WIND_BANK, "FX_WIND", 30);
        self.wind.update(audio, dt, &wind::car(input.speed, levels.wind));
        self.weather.ensure(audio, WIND_BANK, "FX_WIND_Weather", 34);
        self.weather.update(audio, dt, &wind::weather(input.speed, levels.weather));
        if let Some(context) = context {
            for side in 0..2 {
                let [front, rear] = road::WHEELS[side];
                if context.terrain[front].key != context.terrain[rear].key {
                    continue;
                }
                let id = input.wheels[front].road_noise_loop;
                if id == blackbox_carsound::NO_ROAD_NOISE {
                    self.select(audio, side, None);
                    continue;
                }
                if id <= 8 {
                    self.select(audio, side, Some(id));
                }
            }
        }
        let transitions = self.history.observe(input, context);
        for command in commands.iter() {
            match *command {
                SoundCommand::SetLoop { id: LoopId::Road(side), sound: SoundRef::RoadNoise(id), volume, pitch }
                    if id <= 8 =>
                {
                    let side = usize::from(side).min(1);
                    self.select(audio, side, Some(id));
                    self.generated[side] = (volume * 32767.0) as i32;
                    let secondary = context.map_or(id == 0, |c| {
                        let [front, rear] = road::WHEELS[side];
                        c.terrain[front].key == c.terrain[rear].key && c.terrain[front].secondary
                    });
                    self.secondary[side] |= secondary;
                    let level = match id {
                        8 => levels.road_stitch,
                        _ => levels.road[usize::from(id)],
                    };
                    let gain = road::multiply(self.generated[side], level);
                    self.road_params[side] = road::params(
                        id,
                        gain,
                        (pitch * 4096.0) as i32,
                        levels.road_azimuth[side],
                        self.secondary[side],
                        levels.road_wet,
                    );
                }
                SoundCommand::StopLoop(LoopId::Road(side)) => {
                    let side = usize::from(side).min(1);
                    self.generated[side] = 0;
                    self.road_params[side][1] = 0;
                    if road::no_road(input, side) {
                        self.select(audio, side, None);
                    }
                }
                _ => {}
            }
        }
        for (side, transition) in transitions.into_iter().enumerate() {
            if let Some(id) = transition {
                self.transition[side].start(audio, ROAD_BANK, "FX_ROADNOISE_TRANS", 40 + side as u32);
                self.transition[side].params = road::transition(id, input.speed, side, &levels).to_vec();
            }
            let transition_params = self.transition[side].params.clone();
            self.transition[side].update(audio, dt, &transition_params);
            self.road[side].update(audio, dt, &self.road_params[side]);
            let wet_gain = road::multiply(road::multiply(self.generated[side], 24000), levels.road_wet_layer);
            let wet = road::params(4, wet_gain, self.road_params[side][2], levels.road_azimuth[side], false, 0);
            self.wet[side].ensure(audio, ROAD_BANK, "FX_ROADNOISE", 35 + side as u32);
            self.wet[side].update(audio, dt, &wet);
            self.stitches[side].update(
                audio,
                dt,
                self.road_params[side][1],
                levels.road_azimuth[side],
                levels.road_wet,
            );
        }
        commands.retain(|command| match command {
            SoundCommand::SetLoop { id: LoopId::Wind | LoopId::Road(_), .. }
            | SoundCommand::StopLoop(LoopId::Wind | LoopId::Road(_)) => {
                let id = match command {
                    SoundCommand::SetLoop { id, .. } | SoundCommand::StopLoop(id) => *id,
                    _ => unreachable!(),
                };
                !self.owns_loop(id)
            }
            _ => true,
        });
    }

    fn select(&mut self, audio: &mut Audio, side: usize, id: Option<u8>) {
        if self.ids[side] == id {
            return;
        }
        self.ids[side] = id;
        self.secondary[side] = false;
        self.road[side].release();
        self.stitches[side].release();
        let Some(id) = id else { return };
        self.generated[side] = 0;
        self.road_params[side] = road::params(id, 0, 4096, 0, false, 0);
        if id == 8 {
            self.stitches[side].start(audio, 50 + side as u32);
            return;
        }
        self.road[side].start(audio, ROAD_BANK, "FX_ROADNOISE", 31 + side as u32);
        self.road[side].params = self.road_params[side].to_vec();
    }
}
