//! Signed tyre telemetry into the installed FX_SKID graph. Spec: car-audio-tyres-transmission.md.
use blackbox_carsound::{CarInput, LoopId, SoundCommand, WheelInput};
use nfsmw_data::sound::CarSound;

use crate::audio::aems::{EffectModule, EffectSampler};
use crate::audio::{Audio, Group};

/// Raw authored mixer outputs, without an extra generated loop-volume multiplier.
#[derive(Debug, Clone, Copy)]
pub(in crate::audio) struct TyreLevels {
    pub forward: i32,
    pub side: i32,
    pub backward: i32,
    pub azimuth: i32,
    pub pitch_offset: i32,
    pub wet: i32,
}

/// Physics factors and retained side terrain types supplied by their original producers.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::audio) struct TyreFrame {
    pub understeer: f32,
    pub oversteer: f32,
    pub side_surface: [i32; 2],
    pub skid_type: i32,
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn wheel(wheel: WheelInput) -> [f32; 3] {
    let slip = finite(wheel.slip);
    let dead = 0.2 * finite(wheel.tolerated_slip).abs();
    let adjusted = slip.signum() * (slip.abs() - dead).max(0.0);
    [
        (adjusted * 200.0).clamp(-1023.0, 1023.0),
        (-finite(wheel.skid) * 81.0).clamp(-1023.0, 1023.0),
        ((finite(wheel.load) - 4000.0) / 6000.0).clamp(0.0, 1.0) * 1023.0,
    ]
}

/// Signed axle words, shared by the graph parameters and authored dynamic-mixer signals.
pub(in crate::audio) fn axle_words(input: &CarInput) -> [[i32; 3]; 2] {
    if input.wheels_on_ground() == 0 {
        return [[0; 3]; 2];
    }
    [[0, 1], [2, 3]].map(|pair| {
        let [a, b] = pair.map(|i| wheel(input.wheels[i]));
        std::array::from_fn(|i| ((a[i] + b[i]) * 0.5) as i32)
    })
}

/// Authored skid object inputs on the port's sound-controller clock.
#[derive(Debug, Clone, Default)]
pub(in crate::audio) struct TyreMixerSignals {
    clock: f64,
    forward: i32,
    side: i32,
    load: i32,
}

impl TyreMixerSignals {
    pub fn update(&mut self, dt: f32, input: &CarInput) -> [i32; 4] {
        self.clock += f64::from(finite(dt).max(0.0));
        let ticks = ((self.clock * 60.0 + 0.00001).floor() as u64).min(12);
        self.clock = (self.clock - ticks as f64 / 60.0).max(0.0);
        let [front, back] = axle_words(input);
        let forward = (front[0].abs().max(back[0].abs()) << 5).clamp(0, 32767);
        let side = (front[1].abs().max(back[1].abs()) << 5).clamp(0, 32767);
        for _ in 0..ticks {
            self.forward = forward;
            self.side += (side - self.side).clamp(-500, 500);
            self.load += (forward.max(side) - self.load).clamp(-3000, 3000);
        }
        [self.forward, self.side, self.forward, self.load]
    }
}

pub(in crate::audio) fn params(input: &CarInput, frame: TyreFrame, levels: TyreLevels) -> [i32; 22] {
    let [front, back] = axle_words(input);
    [
        levels.forward.clamp(0, 32767),
        levels.side.clamp(0, 32767),
        levels.backward.clamp(0, 32767),
        levels.azimuth,
        frame.skid_type,
        levels.pitch_offset,
        0,
        input.speed_mph() as i32,
        0,
        finite(frame.understeer) as i32,
        finite(frame.oversteer) as i32,
        frame.side_surface[0].max(frame.side_surface[1]),
        front[0],
        front[1],
        front[2],
        back[0],
        back[1],
        back[2],
        25000,
        0,
        32767,
        levels.wet.clamp(0, 32767),
    ]
}

#[derive(Default)]
pub(in crate::audio) struct TyrePlayback {
    module: Option<EffectModule>,
    failed: bool,
}

impl TyrePlayback {
    /// True only when the authored adapter owns skid playback for this update.
    pub fn update(
        &mut self,
        audio: &mut Audio,
        sound: &CarSound,
        input: &CarInput,
        frame: TyreFrame,
        levels: Option<TyreLevels>,
        dt: f32,
    ) -> bool {
        let Some(levels) = levels else {
            self.module = None;
            return false;
        };
        if self.failed {
            return false;
        }
        if self.module.is_none() {
            let Some(bank) = sound.banks.skid_bank() else { return false };
            match EffectModule::load(audio, &format!("SKIDS/{bank}"), "FX_SKID", 41) {
                Ok(module) => self.module = Some(module),
                Err(error) => {
                    log::warn!("authored tyre audio unavailable: {error}");
                    self.failed = true;
                    return false;
                }
            }
        }
        let words = params(input, frame, levels);
        let module = self.module.as_mut().expect("loaded tyre module");
        if module.update(&mut EffectSampler(audio, Group::Sfx), dt, &words) {
            return true;
        }
        self.module = None;
        self.failed = true;
        log::warn!("authored tyre audio failed; restoring generic skid playback");
        false
    }

    /// Invoke only after update succeeds; on failure leave all legacy commands intact.
    pub fn consume(commands: &mut Vec<SoundCommand>) {
        commands.retain(|command| {
            !matches!(
                command,
                SoundCommand::SetLoop { id: LoopId::Skid(_), .. } | SoundCommand::StopLoop(LoopId::Skid(_))
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels() -> TyreLevels {
        TyreLevels { forward: 31000, side: 23000, backward: 19000, azimuth: 500, pitch_offset: 17, wet: 800 }
    }

    #[test]
    fn signed_axle_averages_preserve_cancellation_and_single_grounded_wheel_values() {
        let mut input = CarInput {
            wheels: [WheelInput { on_ground: false, load: 7000.0, ..Default::default() }; 4],
            ..Default::default()
        };
        input.wheels[0].on_ground = true;
        input.wheels[0].slip = 2.0;
        input.wheels[1].slip = -2.0;
        input.wheels[0].skid = 4.0;
        input.wheels[1].skid = -4.0;
        assert_eq!(axle_words(&input)[0], [0, 0, 511]);
        input.wheels[0].on_ground = false;
        assert_eq!(axle_words(&input), [[0; 3]; 2]);
    }

    #[test]
    fn tyre_vector_uses_signed_slip_deadzone_and_raw_mixer_outputs() {
        let mut input = CarInput { speed: 30.0, ..Default::default() };
        input.wheels =
            [WheelInput { slip: -1.0, tolerated_slip: 1.0, skid: 2.0, load: 10000.0, ..Default::default() }; 4];
        let words = params(
            &input,
            TyreFrame { understeer: 3.9, oversteer: -2.9, side_surface: [1, 3], skid_type: 0 },
            levels(),
        );
        assert_eq!(&words[..12], &[31000, 23000, 19000, 500, 0, 17, 0, 67, 0, 3, -2, 3]);
        assert_eq!(&words[12..], &[-160, -162, 1023, -160, -162, 1023, 25000, 0, 32767, 800]);
        input.wheels[0].slip = 1000.0;
        input.wheels[1].slip = 1000.0;
        assert_eq!(axle_words(&input)[0][0], 1023);
        input.wheels[0].slip = f32::NAN;
        input.wheels[1].slip = 0.1;
        assert_eq!(axle_words(&input)[0][0], 0);
    }

    #[test]
    fn successful_adapter_consumes_only_skid_family() {
        let mut commands = vec![SoundCommand::StopLoop(LoopId::Skid(0)), SoundCommand::StopLoop(LoopId::Wind)];
        TyrePlayback::consume(&mut commands);
        assert_eq!(commands, vec![SoundCommand::StopLoop(LoopId::Wind)]);
    }

    #[test]
    fn skid_mixer_inputs_duplicate_forward_and_smooth_slip_instead_of_axle_load() {
        let input = CarInput {
            wheels: [WheelInput { slip: -10.0, skid: 10.0, load: 0.0, ..Default::default() }; 4],
            ..Default::default()
        };
        let mut signals = TyreMixerSignals::default();
        assert_eq!(signals.update(0.0, &input), [0; 4]);
        assert_eq!(signals.update(1.0 / 60.0, &input), [32736, 500, 32736, 3000]);
        assert_eq!(signals.update(1.0 / 60.0, &input), [32736, 1000, 32736, 6000]);
        let airborne = CarInput { wheels: [WheelInput { on_ground: false, ..Default::default() }; 4], ..input };
        assert_eq!(signals.update(1.0 / 60.0, &airborne), [0, 500, 0, 3000]);
    }

    #[test]
    fn signed_skid_signal_smoothing_is_render_rate_independent_and_repays_stall_debt() {
        let input = CarInput {
            wheels: [WheelInput { slip: 5.0, skid: 40.0, load: 10000.0, ..Default::default() }; 4],
            ..Default::default()
        };
        let mut outputs = Vec::new();
        for fps in [5, 60, 144, 360] {
            let mut signals = TyreMixerSignals::default();
            let mut output = [0; 4];
            for _ in 0..fps {
                output = signals.update(1.0 / fps as f32, &input);
            }
            outputs.push(output);
        }
        assert!(outputs.iter().all(|value| *value == outputs[0]));
        assert_eq!(outputs[0][1], 30000, "sixty distinct 500-unit smoothing ticks, below target saturation");
        let mut stalled = TyreMixerSignals::default();
        stalled.update(1.0, &input);
        let mut recovered = [0; 4];
        for _ in 0..4 {
            recovered = stalled.update(0.0, &input);
        }
        assert_eq!(recovered, outputs[0], "zero elapsed time still repays owed controller ticks");
    }

    #[test]
    #[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
    fn installed_tyres_keep_pcm_and_voice_lifecycle_across_render_rates() {
        for rate in [44100usize, 48000] {
            let mut results = Vec::new();
            for fps in [5usize, 60, 144, 360] {
                let dir = game_install::GameDir::open(std::path::PathBuf::from(
                    std::env::var_os("NFSMW_GAME_DIR").expect("requires install"),
                ))
                .unwrap();
                let mut audio = Audio::replay(dir, rate as u32);
                let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
                let input = CarInput {
                    speed: 35.0,
                    wheels: [WheelInput { slip: 5.0, skid: 6.0, load: 10000.0, ..Default::default() }; 4],
                    ..Default::default()
                };
                let mut playback = TyrePlayback::default();
                let (mut energy, mut samples, mut started, mut max_active) = (0.0f64, 0usize, 0u32, 0usize);
                let mut tail_peak = 0.0f32;
                for frame in 0..fps * 4 {
                    if frame < fps * 3 {
                        assert!(playback.update(
                            &mut audio,
                            &sound,
                            &input,
                            TyreFrame::default(),
                            Some(levels()),
                            1.0 / fps as f32
                        ));
                        let counts = playback.module.as_ref().unwrap().voice_counts();
                        started = counts.0;
                        max_active = max_active.max(counts.1);
                    } else {
                        assert!(!playback.update(
                            &mut audio,
                            &sound,
                            &input,
                            TyreFrame::default(),
                            None,
                            1.0 / fps as f32
                        ));
                    }
                    let first = frame * rate / fps;
                    let last = (frame + 1) * rate / fps;
                    let pcm = audio.render_replay(last - first);
                    assert!(pcm.iter().all(|s| s.is_finite()));
                    for (index, sample) in pcm.iter().enumerate() {
                        let clock = first + index / 2;
                        if (rate..rate * 3).contains(&clock) {
                            energy += f64::from(*sample).powi(2);
                            samples += 1;
                        }
                        if clock >= rate * 7 / 2 {
                            tail_peak = tail_peak.max(sample.abs());
                        }
                    }
                }
                let rms = (energy / samples as f64).sqrt();
                assert!(rms > 0.0001, "tyres {rate}/{fps}: {rms}");
                assert!(started > 0 && max_active <= 7, "bounded authored skid voices");
                assert!(tail_peak < 0.0001, "dropped tyre module must release every voice: {tail_peak}");
                assert!(audio.missing.is_empty());
                results.push((fps, rms, started));
            }
            let baseline = results[1];
            for (fps, rms, started) in results {
                assert!(
                    (0.85..=1.15).contains(&(rms / baseline.1)),
                    "tyres {rate}/{fps} RMS {rms}, 60FPS {}",
                    baseline.1
                );
                assert!(started.abs_diff(baseline.2) <= 2, "tyres {rate}/{fps} starts {started}, 60FPS {}", baseline.2);
            }
        }
    }

    #[test]
    #[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
    fn installed_tyres_render_and_missing_mixer_restores_fallback() {
        let dir = game_install::GameDir::open(std::path::PathBuf::from(
            std::env::var_os("NFSMW_GAME_DIR").expect("requires install"),
        ))
        .unwrap();
        let mut audio = Audio::replay(dir, 48000);
        let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
        let input = CarInput {
            speed: 35.0,
            wheels: [WheelInput { slip: 5.0, skid: 6.0, load: 10000.0, ..Default::default() }; 4],
            ..Default::default()
        };
        let mut playback = TyrePlayback::default();
        let mut peak = 0.0f32;
        for _ in 0..180 {
            assert!(playback.update(&mut audio, &sound, &input, TyreFrame::default(), Some(levels()), 1.0 / 60.0));
            let pcm = audio.render_replay(800);
            assert!(pcm.iter().all(|s| s.is_finite()));
            peak = pcm.iter().fold(peak, |peak, s| peak.max(s.abs()));
        }
        assert!(peak > 0.0001, "installed graph must request audible samples");
        assert!(!playback.update(&mut audio, &sound, &input, TyreFrame::default(), None, 1.0 / 60.0));
        assert!(playback.module.is_none());
        let mut broken = sound;
        broken.banks.skids = vec!["missing-tyre-bank.abk".into()];
        let mut fallback = TyrePlayback::default();
        let mut commands = vec![SoundCommand::StopLoop(LoopId::Skid(0))];
        if fallback.update(&mut audio, &broken, &input, TyreFrame::default(), Some(levels()), 1.0 / 60.0) {
            TyrePlayback::consume(&mut commands);
        }
        assert!(fallback.failed);
        assert_eq!(
            commands,
            vec![SoundCommand::StopLoop(LoopId::Skid(0))],
            "failed authored load must retain fallback commands"
        );
    }
}
