//! Ordinary CAR_TRANNY playback, distinct from reverse whine. Spec: car-audio-tyres-transmission.md.
use nfsmw_data::sound::CarSound;

use crate::audio::aems::{EffectModule, EffectSampler};
use crate::audio::{Audio, Group};

#[derive(Debug, Clone, Copy)]
pub(in crate::audio) struct TransmissionLevels {
    pub volume: i32,
    pub azimuth: i32,
    pub pitch_offset: i32,
    pub low_pass: i32,
    pub wet: i32,
    pub dry: i32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::audio) struct TransmissionFrame {
    pub forward_speed: f32,
    pub shifting: bool,
}

pub(in crate::audio) fn params(class: u32, frame: TransmissionFrame, levels: TransmissionLevels) -> [i32; 9] {
    let speed = if frame.forward_speed.is_finite() { frame.forward_speed } else { 0.0 };
    [
        class as i32,
        ((speed * 15.0) as i32).clamp(0, 10000),
        levels.volume.clamp(0, 32787),
        levels.azimuth.clamp(0, 65536),
        levels.pitch_offset.clamp(-16383, 16383),
        levels.low_pass.clamp(0, 25000),
        levels.wet.clamp(0, 32767),
        levels.dry.clamp(0, 32767),
        i32::from(frame.shifting),
    ]
}

#[derive(Default)]
pub(in crate::audio) struct TransmissionPlayback {
    module: Option<EffectModule>,
    failed: bool,
}

impl TransmissionPlayback {
    pub fn update(
        &mut self,
        audio: &mut Audio,
        sound: &CarSound,
        frame: TransmissionFrame,
        levels: Option<TransmissionLevels>,
        dt: f32,
    ) -> bool {
        if !sound.engine.has_transmission_loop || levels.is_none() {
            self.module = None;
            return false;
        }
        if self.failed {
            return false;
        }
        if self.module.is_none() {
            match EffectModule::load(audio, "ENGINE/CAR_TRANNY.abk", "CAR_TRANNY", 42) {
                Ok(module) => self.module = Some(module),
                Err(error) => {
                    log::warn!("authored transmission audio unavailable: {error}");
                    self.failed = true;
                    return false;
                }
            }
        }
        let words = params(sound.engine.car_id, frame, levels.expect("present transmission levels"));
        if self.module.as_mut().expect("loaded transmission module").update(
            &mut EffectSampler(audio, Group::Sfx),
            dt,
            &words,
        ) {
            return true;
        }
        self.module = None;
        self.failed = true;
        log::warn!("authored transmission audio failed");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels() -> TransmissionLevels {
        TransmissionLevels { volume: 28000, azimuth: 1000, pitch_offset: 30, low_pass: 22000, wet: 10000, dry: 26000 }
    }

    #[test]
    fn transmission_uses_signed_projection_then_pc_setter_bounds_and_shifting_state() {
        assert_eq!(
            params(66, TransmissionFrame { forward_speed: -12.39, shifting: true }, levels()),
            [66, 0, 28000, 1000, 30, 22000, 10000, 26000, 1]
        );
        assert_eq!(params(66, TransmissionFrame { forward_speed: 12.39, shifting: false }, levels())[1], 185);
        let extreme = TransmissionLevels { volume: 40000, azimuth: 70000, pitch_offset: -20000, ..levels() };
        assert_eq!(
            &params(66, TransmissionFrame { forward_speed: 1000.0, shifting: false }, extreme)[1..5],
            &[10000, 32787, 65536, -16383]
        );
        assert_eq!(params(66, TransmissionFrame { forward_speed: f32::NAN, shifting: false }, levels())[1], 0);
    }

    #[test]
    #[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
    fn installed_transmission_keeps_pcm_and_voices_across_render_rates() {
        for rate in [44100usize, 48000] {
            let mut results = Vec::new();
            for fps in [5usize, 60, 144, 360] {
                let dir = game_install::GameDir::open(std::path::PathBuf::from(
                    std::env::var_os("NFSMW_GAME_DIR").expect("requires install"),
                ))
                .unwrap();
                let mut audio = Audio::replay(dir, rate as u32);
                let sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
                assert!(sound.engine.has_transmission_loop);
                let mut playback = TransmissionPlayback::default();
                let (mut energy, mut samples, mut started, mut max_active) = (0.0f64, 0usize, 0u32, 0usize);
                let mut tail_peak = 0.0f32;
                for frame in 0..fps * 4 {
                    let telemetry = TransmissionFrame { forward_speed: 50.0, shifting: false };
                    let selected = (frame < fps * 3).then_some(levels());
                    let active = playback.update(&mut audio, &sound, telemetry, selected, 1.0 / fps as f32);
                    assert_eq!(active, frame < fps * 3);
                    if active {
                        let counts = playback.module.as_ref().unwrap().voice_counts();
                        started = counts.0;
                        max_active = max_active.max(counts.1);
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
                assert!(rms > 0.0001, "transmission {rate}/{fps}: {rms}");
                assert!(started > 0 && max_active <= 3, "bounded authored transmission voices");
                assert!(tail_peak < 0.0001, "dropped transmission must release every voice: {tail_peak}");
                assert!(audio.missing.is_empty());
                results.push((fps, rms, started));
            }
            let baseline = results[1];
            for (fps, rms, started) in results {
                assert!(
                    (0.85..=1.15).contains(&(rms / baseline.1)),
                    "transmission {rate}/{fps} RMS {rms}, 60FPS {}",
                    baseline.1
                );
                assert!(
                    started.abs_diff(baseline.2) <= 2,
                    "transmission {rate}/{fps} starts {started}, 60FPS {}",
                    baseline.2
                );
            }
        }
    }

    #[test]
    #[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
    fn installed_transmission_runs_in_forward_reverse_and_shift_then_drops() {
        let dir = game_install::GameDir::open(std::path::PathBuf::from(
            std::env::var_os("NFSMW_GAME_DIR").expect("requires install"),
        ))
        .unwrap();
        let mut audio = Audio::replay(dir, 48000);
        let mut sound = audio.load_car_engine("BMWM3GTR").unwrap().sound;
        assert!(sound.engine.has_transmission_loop, "fixture must actually enable Tranny");
        let mut playback = TransmissionPlayback::default();
        let mut peak = 0.0f32;
        for tick in 0..180 {
            let frame = TransmissionFrame {
                forward_speed: if tick < 120 { 50.0 } else { -20.0 },
                shifting: (60..90).contains(&tick),
            };
            assert!(playback.update(&mut audio, &sound, frame, Some(levels()), 1.0 / 60.0));
            let pcm = audio.render_replay(800);
            assert!(pcm.iter().all(|s| s.is_finite()));
            peak = pcm.iter().fold(peak, |peak, s| peak.max(s.abs()));
        }
        assert!(peak > 0.0001, "installed module must render its samples");
        sound.engine.has_transmission_loop = false;
        assert!(!playback.update(&mut audio, &sound, TransmissionFrame::default(), Some(levels()), 1.0 / 60.0));
        assert!(playback.module.is_none());
    }
}
