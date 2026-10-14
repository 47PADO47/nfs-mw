//! Authored wind and ordinary road-loop graphs, retaining voices across updates.
use crate::audio::{
    Audio, Group,
    aems::{EffectModule, EffectSampler},
    mixer::ambient::AmbientLevels,
};
use blackbox_carsound::{CarInput, LoopId, SoundCommand, SoundRef};

#[derive(Default)]
struct Slot {
    module: Option<EffectModule>,
    failed: bool,
}

impl Slot {
    fn update(&mut self, audio: &mut Audio, bank: &str, class: &str, seed: u32, dt: f32, params: &[i32]) {
        if self.failed {
            return;
        }
        if self.module.is_none() {
            match EffectModule::load(audio, bank, class, seed) {
                Ok(module) => self.module = Some(module),
                Err(error) => {
                    log::warn!("{class} unavailable: {error}");
                    self.failed = true;
                    return;
                }
            }
        }
        if self.module.as_mut().is_some_and(|m| !m.update(&mut EffectSampler(audio, Group::Sfx), dt, params)) {
            self.module = None;
            self.failed = true;
        }
    }
}

#[derive(Default)]
pub(super) struct AmbientPlayback {
    wind: Slot,
    road: [Slot; 2],
    road_params: [[i32; 11]; 2],
}

impl AmbientPlayback {
    #[cfg(test)]
    pub fn available(&self) -> bool {
        !self.wind.failed && self.wind.module.is_some() && self.road.iter().all(|s| !s.failed && s.module.is_some())
    }
    pub fn update(
        &mut self,
        audio: &mut Audio,
        input: &CarInput,
        commands: &mut Vec<SoundCommand>,
        levels: AmbientLevels,
        dt: f32,
    ) {
        let speed = input.speed.abs();
        let speed = match speed {
            v if !v.is_finite() => 2.0,
            v if v > 40.0 => 39.95,
            v => v.max(2.0),
        };
        let ratio = speed / 40.0;
        let volume = (ratio * 32767.0) as i32;
        let [left_az, right_az, left, right, rumble, pitch] = levels.wind;
        let multiply = |a: i32, b: i32| ((i64::from(a) * i64::from(b)) >> 15) as i32;
        let wind = [
            pitch,
            multiply(left, volume),
            multiply(right, volume),
            left_az,
            right_az,
            (ratio * 1040.0) as i32,
            0,
            0,
            multiply(rumble, volume),
            25000,
            0,
            32767,
            0,
            15,
        ];
        self.wind.update(audio, "IG_GLOBAL/WIND_00_MB.abk", "FX_WIND", 30, dt, &wind);
        commands.retain(|command| match *command {
            SoundCommand::SetLoop { id: LoopId::Wind, .. } | SoundCommand::StopLoop(LoopId::Wind) => false,
            SoundCommand::SetLoop { id: LoopId::Road(side), sound: SoundRef::RoadNoise(id), volume, pitch }
                if id < 8 =>
            {
                let side = usize::from(side).min(1);
                self.road_params[side] = [
                    i32::from(id),
                    multiply((volume * 32767.0) as i32, levels.road[usize::from(id)]),
                    (pitch * 4096.0) as i32,
                    levels.road_azimuth[side],
                    0,
                    i32::from(id == 0),
                    0,
                    25000,
                    0,
                    32767,
                    levels.road_wet,
                ];
                false
            }
            SoundCommand::StopLoop(LoopId::Road(side)) => {
                self.road_params[usize::from(side).min(1)][1] = 0;
                false
            }
            SoundCommand::SetLoop { id: LoopId::Road(side), .. } => {
                self.road_params[usize::from(side).min(1)][1] = 0;
                true
            }
            _ => true,
        });
        for side in 0..2 {
            if self.road_params[side][1] == 0 && self.road[side].module.is_none() {
                continue;
            }
            self.road[side].update(
                audio,
                "IG_GLOBAL/ROADNOISE_00_MB.abk",
                "FX_ROADNOISE",
                31 + side as u32,
                dt,
                &self.road_params[side],
            );
        }
    }
}
