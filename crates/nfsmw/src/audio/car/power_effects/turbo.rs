//! Forced-induction parameters and lifecycle; spec: car-audio-power-effects.md.
use nfsmw_data::sound::{CarSound, TurboSound};

use super::{PowerFrame, PowerLevels, Running, params};
use crate::audio::{Audio, Group, aems::EffectModule};

const TICK: f32 = 1.0 / 60.0;

#[derive(Default)]
pub(super) struct TurboPlayback {
    spool: Option<Running>,
    blowoff: Option<Running>,
    failed: bool,
    charge: f32,
    percent: f32,
    volume: i32,
    reached_peak: bool,
    peak_time: f32,
    clock: f32,
    blowoff_volume: i32,
    fade_age: Option<f32>,
    seed: u32,
}

impl TurboPlayback {
    pub fn owns(&self) -> bool {
        self.spool.is_some() && !self.failed
    }

    fn start(&mut self, audio: &mut Audio, sound: &TurboSound, words: [i32; 6]) -> Option<Running> {
        self.seed = self.seed.wrapping_add(1);
        match EffectModule::load(audio, &format!("TURBO/{}", sound.bank), "FX_TURBO_01", self.seed) {
            Ok(module) => Some(Running { module, words: words.to_vec(), group: Group::Engine }),
            Err(error) => {
                log::warn!("authored turbo audio unavailable: {error}");
                self.failed = true;
                None
            }
        }
    }

    pub fn play(&mut self, audio: &mut Audio, car: &CarSound, frame: PowerFrame, levels: PowerLevels, id: u8) -> bool {
        if self.failed {
            return false;
        }
        if self.blowoff.is_some() {
            return true;
        }
        let Some(sound) = car.turbo.as_ref() else { return false };
        let id = i32::from(id.min(2)) + 1;
        // The native producer leaks once before testing the falling-torque blowoff edge.
        let percent = match frame.engine_torque < 20.0 && sound.charge_time > 0.0 {
            true => (self.charge - sound.leak_rate).max(0.0) / sound.charge_time,
            false => self.percent,
        };
        let volume = sound.blowoff_volume[usize::from(id != 1)] as f32 * percent;
        self.blowoff_volume = volume as i32;
        let gain = params::q15(self.blowoff_volume, levels.turbo_volume[usize::from(id != 1) + 1]);
        self.blowoff = self.start(
            audio,
            sound,
            params::turbo(
                id,
                gain,
                (percent * 1024.0) as i32,
                levels.turbo_azimuth,
                frame.exhaust_rotation,
                frame.physics_rpm,
            ),
        );
        if let Some(running) = self.blowoff.as_mut() {
            running.group = Group::Sfx;
        }
        self.charge = 0.0;
        self.reached_peak = false;
        self.fade_age = None;
        self.blowoff.is_some()
    }

    pub fn update(&mut self, audio: &mut Audio, car: &CarSound, frame: PowerFrame, levels: PowerLevels, dt: f32) {
        let Some(sound) = car.turbo.as_ref() else { return };
        if self.failed {
            return;
        }
        if self.spool.is_none() {
            self.spool = self.start(audio, sound, params::turbo(0, 0, 0, levels.turbo_azimuth, 0, frame.physics_rpm));
        }
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.advance(sound, frame, dt);
        if let Some(running) = self.spool.as_mut() {
            let gain = params::q15(self.volume, levels.turbo_volume[0].clamp(0, 32767));
            running.words = params::turbo(
                0,
                gain,
                (self.percent * 1024.0) as i32,
                levels.turbo_azimuth,
                (frame.physics_torque * 10.24) as i32,
                frame.physics_rpm,
            )
            .to_vec();
            if !running.update(audio, dt) {
                self.spool = None;
                self.failed = true;
            }
        }
        if let Some(age) = self.fade_age.as_mut() {
            *age += dt;
        }
        if self.fade_age.is_none() && frame.physics_torque > 20.0 && !frame.shifting && self.blowoff.is_some() {
            self.fade_age = Some(0.0);
        }
        let factor = self.fade_age.map_or(1.0, |age| (1.0 - age / 0.15).clamp(0.0, 1.0));
        if factor <= 0.0 {
            self.blowoff = None;
            return;
        }
        if let Some(running) = self.blowoff.as_mut() {
            let mixer = levels.turbo_volume[usize::from(running.words[0] != 1) + 1].clamp(0, 32767);
            running.words[1] = (params::q15(self.blowoff_volume, mixer) as f32 * factor) as i32;
            running.words[3] = levels.turbo_azimuth.clamp(0, 65535);
            if !running.update(audio, dt) {
                if !running.module.finished() {
                    self.failed = true;
                }
                self.blowoff = None;
            }
        }
    }

    fn advance(&mut self, sound: &TurboSound, frame: PowerFrame, dt: f32) {
        self.clock += dt;
        let mut ticks = 0;
        while self.clock + 0.000001 >= TICK && ticks < 12 {
            self.clock = (self.clock - TICK).max(0.0);
            self.tick(sound, frame);
            ticks += 1;
        }
    }

    fn tick(&mut self, sound: &TurboSound, frame: PowerFrame) {
        if !(sound.charge_time.is_finite() && sound.charge_time > 0.0) {
            self.percent = 0.0;
            self.volume = 0;
            return;
        }
        let charge = match frame.engine_torque > 20.0 {
            true => frame.engine_torque * 0.01,
            false => -sound.leak_rate,
        };
        self.charge = (self.charge + charge).clamp(0.0, sound.charge_time);
        let rpm = frame.physics_rpm as i32;
        let limit = sound.charge_time * (((rpm - 1000) as f32 * 0.01).clamp(0.0, 1.0) * 0.6 + 0.4);
        self.charge = self.charge.min(limit);
        let arm = self.charge == limit && !self.reached_peak;
        if arm {
            self.reached_peak = true;
            self.peak_time = 0.0;
        }
        if !arm && self.charge != sound.charge_time && self.reached_peak {
            self.reached_peak = false;
        }
        self.percent = self.charge / sound.charge_time;
        self.volume = sound.spool_volume.min(i32::MAX as u32) as i32;
        if !self.reached_peak {
            return;
        }
        self.peak_time += TICK;
        let index = (((self.peak_time - 1.0).max(0.0) / 2.0).min(1.0) * 307.2) as i32;
        self.volume = params::q15(self.volume, params::cosine(index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound() -> TurboSound {
        TurboSound {
            name: String::new(),
            bank: String::new(),
            spool_volume: 20000,
            charge_time: 20.0,
            leak_rate: 0.5,
            blowoff_volume: [20000, 20000],
        }
    }

    #[test]
    fn spool_charge_duck_and_elapsed_debt_are_independent_of_render_rate() {
        let frame = PowerFrame { physics_rpm: 7500.0, engine_torque: 100.0, ..Default::default() };
        let mut outputs = Vec::new();
        for fps in [5usize, 60, 144, 360] {
            let mut spool = TurboPlayback::default();
            for _ in 0..fps * 4 {
                spool.advance(&sound(), frame, 1.0 / fps as f32);
            }
            assert_eq!(spool.percent, 1.0);
            outputs.push(spool.volume);
        }
        assert!(outputs.iter().all(|volume| *volume == outputs[0]), "{outputs:?}");
        assert!(outputs[0] < 14000 && outputs[0] > 10000);
        let mut stalled = TurboPlayback::default();
        stalled.advance(&sound(), frame, 0.5);
        assert!(stalled.clock > 0.29, "keep capped tick debt");
        stalled.advance(&sound(), frame, 0.0);
        stalled.advance(&sound(), frame, 0.0);
        assert!(stalled.clock < 0.001);
        assert_eq!(stalled.percent, 1.0);
    }

    #[test]
    fn partial_spool_uses_authored_psi_instead_of_multiplying_volume_again() {
        let mut spool = TurboPlayback::default();
        spool.tick(&sound(), PowerFrame { physics_rpm: 5000.0, engine_torque: 100.0, ..Default::default() });
        assert_eq!(spool.percent, 0.05);
        assert_eq!(spool.volume, 20000);
    }
}
