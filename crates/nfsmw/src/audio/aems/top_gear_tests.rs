//! Installed PC highest-gear regression with the reported fully upgraded Lotus.
use blackbox_carsound::{CarInput, EngineMixer};
use game_install::GameDir;
use kira::{Frame, sound::SoundData};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory, PhysicsData};
use nfsmw_data::sound::SoundUpgrades;

use super::soft::{RATE, Soft, SoftSampler};
use super::{AemsLayer, Feed};
use crate::audio::mixer::CarMixer;
use crate::audio::tuning::tuning;
use crate::audio::{Audio, EngineMix, EngineVoice, LoopMix, Volumes};

#[test]
#[ignore = "requires installed Lotus physics and audio; set NFSMW_GAME_DIR"]
fn upgraded_lotus_at_366_kmh_excludes_only_the_highest_gear_limiter_sample() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR is required")).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let catalog = PerformanceCatalog::read(physics.database(), "elise").unwrap();
    let mut package = catalog.maximum();
    for category in PerformanceCategory::ALL {
        if catalog.junkman_available(category, package) {
            package.junkman |= 1 << category as u8;
        }
    }
    let spec = physics.car_with_performance("elise", package).unwrap().spec;
    let top_gear = spec.transmission.top_gear() as i32;
    assert!(top_gear > blackbox_carsound::GEAR_FIRST);
    let upgrades = SoundUpgrades {
        engine: i32::from(package.get(PerformanceCategory::Engine)),
        transmission: i32::from(package.get(PerformanceCategory::Transmission)),
        induction: i32::from(package.get(PerformanceCategory::Induction)),
    };
    let mut audio = Audio::new(dir, Volumes::default());
    for gear in [top_gear, top_gear - 1] {
        let loaded = audio.load_car_engine_upgraded("ELISE", upgrades).unwrap();
        let mut controller = EngineMixer::new(&tuning(&loaded.sound, loaded.accel_min_frequency()));
        let mut reference = controller.clone();
        let initial = EngineMix::shared(loaded.sound.engine.min_rpm, 1.0, 0.0, 0.0);
        let (mut sound, handle) = EngineVoice { start: initial, ..loaded.voice }.into_sound().unwrap();
        let mut layer = AemsLayer::load(&mut audio, &loaded.sound).expect("installed CAR graph");
        let mut dynamic = CarMixer::load(&audio.dir, !loaded.sound.engine.decel_loop.is_empty()).unwrap();
        let mut soft = Soft::default();
        let info = kira::info::MockInfoBuilder::new().build();
        let mut frames = vec![Frame::ZERO; (RATE / 60.0) as usize];
        let mut samples = vec![0.0; frames.len()];
        let mut loops_energy = 0.0f64;
        let mut total_energy = 0.0f64;
        let highest = gear == top_gear;
        for tick in 0..600 {
            let input = CarInput {
                rpm_pct: 1.0,
                throttle: 1.0,
                gear,
                top_gear: Some(top_gear),
                speed: 366.0 / 3.6,
                ..Default::default()
            };
            controller.set_redline_sample_available(layer.engine_available());
            let out = controller.update(1.0 / 60.0, &input);
            let old = reference.update(1.0 / 60.0, &CarInput { top_gear: None, ..input });
            if !highest {
                assert_eq!(out, old, "supplying highest gear preserves every lower-gear control value");
            }
            let levels = dynamic.update(
                1.0 / 60.0,
                &crate::audio::mixer::Frame {
                    input: &input,
                    engine: &out,
                    effects: Default::default(),
                    master_volume: loaded.sound.engine.master_volume,
                    sparks: layer.sparks(),
                },
            );
            handle.set(EngineMix {
                accel: LoopMix {
                    frequency: out.accel_loop.frequency,
                    volume: out.accel_volume * levels.engine_volume,
                    pitch: out.accel_loop.playback_rate * levels.engine_pitch,
                },
                decel: LoopMix {
                    frequency: out.decel_loop.frequency,
                    volume: out.decel_volume * levels.engine_volume,
                    pitch: out.decel_loop.playback_rate * levels.engine_pitch,
                },
                low_pass_hz: out.low_pass_hz,
                ..Default::default()
            });
            layer.update(
                &mut SoftSampler { audio: &mut audio, mix: &mut soft },
                1.0 / 60.0,
                &Feed { engine: &out, levels: &levels },
            );
            sound.process(&mut frames, 1.0 / RATE, &info);
            samples.fill(0.0);
            soft.render(&mut samples);
            if tick < 240 {
                continue;
            }
            assert_eq!(out.redlining, !highest);
            assert!(layer.engine_available(), "the installed layer stays usable");
            if highest {
                assert_eq!(out.redline_sample_volume, 0.0);
                assert!(old.redline_sample_volume > 0.0, "the preceding behavior reproduced takeover");
                assert!(out.accel_volume > old.accel_volume * 6.0);
            }
            let mut audible = false;
            for (frame, sample) in frames.iter().zip(&samples) {
                let total = frame.left + sample;
                assert!(total.is_finite());
                audible |= total.abs() > 1e-6;
                loops_energy += f64::from(frame.left).powi(2);
                total_energy += f64::from(total).powi(2);
            }
            assert!(audible, "no silent complete-engine blocks");
        }
        assert!(loops_energy > 0.0 && total_energy > 0.0);
        eprintln!(
            "ELISE upgrades {upgrades:?}, highest {top_gear}, gear {gear}: loops {loops_energy}, total {total_energy}"
        );
    }
}
