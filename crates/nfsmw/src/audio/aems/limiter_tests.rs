//! Sustained limiter coverage with the real banks, mixer map and Ginsu voices. No output device is needed.

use blackbox_carsound::{CarInput, EngineMixer, GEAR_FIRST};
use game_install::GameDir;
use kira::Frame;
use kira::sound::SoundData;

use super::host::{Sampler, Voice};
use super::soft::{RATE, Soft, SoftSampler};
use super::{AemsLayer, Feed};
use crate::audio::mixer::{CarMixer, Levels};
use crate::audio::tuning::tuning;
use crate::audio::{Audio, EngineMix, EngineVoice, LoopMix, Volumes};

const TICK: f32 = 1.0 / 60.0;
const STOCK: &[&str] = &["BMWM3GTR", "PUNTO", "MUSTANGGT", "CORVETTE", "CARRERAGT", "LANCEREVO8"];

#[derive(Debug, Clone, Copy)]
enum SamplePath {
    Healthy,
    Missing,
    Lost,
    Destroyed,
}

/// A real module whose newly requested bank recordings become unavailable at the limiter.
struct MissingRecording<'a>(SoftSampler<'a>);

impl Sampler for MissingRecording<'_> {
    fn sound(&mut self, _: &str, _: usize) -> Result<kira::sound::static_sound::StaticSoundData, String> {
        Err("injected unavailable recording".into())
    }

    fn start(
        &mut self,
        data: kira::sound::static_sound::StaticSoundData,
        gain: f32,
        rate: f32,
    ) -> Option<Box<dyn Voice>> {
        self.0.start(data, gain, rate)
    }
}

#[derive(Default)]
struct Meter {
    energy: f64,
    samples: usize,
    peak: f32,
    silent_blocks: usize,
}

impl Meter {
    fn record(&mut self, samples: &[f32]) {
        assert!(samples.iter().all(|s| s.is_finite()), "finite PCM");
        self.energy += samples.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>();
        self.samples += samples.len();
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        self.peak = self.peak.max(peak);
        self.silent_blocks += usize::from(peak <= 1e-6);
    }

    fn rms(&self) -> f64 {
        (self.energy / self.samples.max(1) as f64).sqrt()
    }
}

#[derive(Default)]
struct Stage {
    total: Meter,
    sample_layer: Meter,
    loops: Meter,
}

fn input(t: f32) -> CarInput {
    let rpm_pct = match t {
        t if t < 1.0 => 0.0,
        t if t < 4.0 => 0.94 * (t - 1.0) / 3.0,
        t if t < 6.0 => 0.94,
        t if t < 14.0 => 1.0,
        _ => 0.8,
    };
    CarInput { rpm_pct, throttle: f32::from(u8::from(t >= 1.0)), gear: GEAR_FIRST, speed: 30.0, ..Default::default() }
}

fn drive(audio: &mut Audio, car: &str, path: SamplePath) -> [Stage; 3] {
    let loaded = audio.load_car_engine(car).unwrap_or_else(|e| panic!("{car}: {e}"));
    let mut engine = EngineMixer::new(&tuning(&loaded.sound, loaded.accel_min_frequency()));
    let silent = EngineMix::shared(loaded.sound.engine.min_rpm, 1.0, 0.0, 0.0);
    let (mut sound, handle) = EngineVoice { start: silent, ..loaded.voice }.into_sound().unwrap();
    let info = kira::info::MockInfoBuilder::new().build();
    let dual = !loaded.sound.engine.decel_loop.is_empty();
    let mut dynamic = CarMixer::load(&audio.dir, dual).expect("mixer map");
    let mut layer = match path {
        SamplePath::Missing => None,
        _ => Some(AemsLayer::load(audio, &loaded.sound).expect("CAR bank module")),
    };
    let mut soft = Soft::default();
    let mut levels = Levels::unmixed();
    let mut stages: [Stage; 3] = std::array::from_fn(|_| Stage::default());
    let (mut block, mut samples, mut loops, mut total) = (
        vec![Frame::ZERO; (RATE / 60.0) as usize],
        vec![0.0; (RATE / 60.0) as usize],
        vec![0.0; (RATE / 60.0) as usize],
        vec![0.0; (RATE / 60.0) as usize],
    );
    for tick in 0..17 * 60 {
        let t = tick as f32 * TICK;
        let mut input = input(t);
        input.pitch_multiplier = levels.engine_pitch;
        engine.set_redline_sample_available(layer.as_ref().is_some_and(AemsLayer::engine_available));
        let out = engine.update(TICK, &input);
        handle.set(EngineMix {
            accel: LoopMix {
                frequency: out.accel_loop.frequency,
                volume: out.accel_volume * levels.engine_volume,
                pitch: out.accel_loop.playback_rate,
            },
            decel: LoopMix {
                frequency: out.decel_loop.frequency,
                volume: out.decel_volume * levels.engine_volume,
                pitch: out.decel_loop.playback_rate,
            },
        });
        levels = dynamic.update(
            TICK,
            &crate::audio::mixer::Frame {
                input: &input,
                engine: &out,
                effects: Default::default(),
                master_volume: loaded.sound.engine.master_volume,
                sparks: layer.as_ref().is_some_and(AemsLayer::sparks),
            },
        );
        if let Some(layer) = layer.as_mut() {
            let mut sampler = SoftSampler { audio, mix: &mut soft };
            if matches!(path, SamplePath::Destroyed) && tick == 8 * 60 {
                let part = layer.engine.as_mut().expect("CAR module");
                part.instance.destroy(&mut super::PartHost {
                    sampler: &mut sampler,
                    bank: &part.bank,
                    voices: &mut part.voices,
                    makeup: levels.makeup,
                });
            }
            let feed = Feed { engine: &out, levels: &levels };
            match path {
                SamplePath::Lost if t >= 6.0 => layer.update(&mut MissingRecording(sampler), TICK, &feed),
                _ => layer.update(&mut sampler, TICK, &feed),
            }
        }
        sound.process(&mut block, 1.0 / RATE, &info);
        samples.fill(0.0);
        soft.render(&mut samples);
        for (((frame, sample), loop_sample), sum) in block.iter().zip(&samples).zip(&mut loops).zip(&mut total) {
            *loop_sample = frame.left;
            *sum = frame.left + sample;
        }
        if t >= 4.0 {
            assert!(total.iter().all(|s| s.is_finite()), "{car}, t {t}: finite output through transitions");
            assert!(total.iter().any(|s| s.abs() > 1e-6), "{car}, t {t}: no dropout through limiter/recovery");
        }
        let stage = match t {
            t if (5.0..6.0).contains(&t) => 0,
            t if (12.0..14.0).contains(&t) => 1,
            t if (15.0..17.0).contains(&t) => 2,
            _ => continue,
        };
        let expected_available = match path {
            SamplePath::Healthy => true,
            SamplePath::Missing => false,
            SamplePath::Lost | SamplePath::Destroyed => stage == 0,
        };
        assert_eq!(layer.as_ref().is_some_and(AemsLayer::engine_available), expected_available, "{car}, t {t}");
        assert_eq!(out.redlining, stage == 1, "{car}, t {t}, audio RPM {}", out.eng_rpm);
        stages[stage].total.record(&total);
        stages[stage].sample_layer.record(&samples);
        stages[stage].loops.record(&loops);
    }
    eprintln!("{car} {path:?}: {} ({})", loaded.sound.engine.name, loaded.sound.engine.bank_main);
    stages
}

fn report(car: &str, stages: &[Stage; 3]) {
    for (name, stage) in ["high RPM", "limiter", "recovery"].into_iter().zip(stages) {
        eprintln!(
            "  {name}: RMS {:.5}, loops {:.5}, sample layer {:.5}, peak {:.3}, silent blocks {}",
            stage.total.rms(),
            stage.loops.rms(),
            stage.sample_layer.rms(),
            stage.total.peak,
            stage.total.silent_blocks
        );
        assert!(stage.total.rms() > 0.005, "{car} {name}: audible engine");
        assert_eq!(stage.total.silent_blocks, 0, "{car} {name}: no full-block dropouts");
    }
    assert!(stages[1].total.rms() > stages[0].total.rms() * 0.1, "{car}: limiter is not effectively muted");
}

fn audio() -> Audio {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR is required");
    let dir = GameDir::open(std::path::PathBuf::from(path)).unwrap();
    Audio::new(dir, Volumes::default())
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn sustained_limiter_and_recovery_keep_the_complete_engine_audible() {
    let mut audio = audio();
    let names: Vec<String> = std::env::var("NFSMW_LIMITER_CARS")
        .ok()
        .map(|names| names.split(',').map(str::trim).filter(|n| !n.is_empty()).map(str::to_owned).collect())
        .unwrap_or_else(|| STOCK.iter().map(|&n| n.to_owned()).collect());
    assert!(!names.is_empty(), "at least one car");
    for car in names {
        let stages = drive(&mut audio, &car, SamplePath::Healthy);
        report(&car, &stages);
        assert!(stages[1].sample_layer.rms() > 0.001, "{car}: sample layer remains audible at the limiter");
    }
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn missing_and_failed_sample_layers_keep_real_engine_loops_audible() {
    let mut audio = audio();
    for path in [SamplePath::Missing, SamplePath::Lost, SamplePath::Destroyed] {
        let stages = drive(&mut audio, "BMWM3GTR", path);
        report("BMWM3GTR", &stages);
        assert!(stages[1].loops.rms() > 0.05, "the real Ginsu loop keeps its normal level without a replacement");
        if matches!(path, SamplePath::Missing | SamplePath::Destroyed) {
            assert_eq!(stages[1].sample_layer.rms(), 0.0);
        }
    }
}
