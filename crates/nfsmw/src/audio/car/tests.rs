//! Against the install: the real data drives the mixers (needs `NFSMW_GAME_DIR`; passes without checking otherwise).

use blackbox_carsound::{GEAR_FIRST, SweetenerKind};
use game_install::GameDir;
use kira::Frame;
use kira::sound::SoundData;

use super::*;
use crate::audio::LoopMix;
use crate::audio::aems::soft::{Soft, SoftSampler};
use crate::audio::{EngineVoice, Volumes};

const RATE: f64 = 48_000.0;
const TICK: f32 = 1.0 / 60.0;

fn audio() -> Option<Audio> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(Audio::new(GameDir::open(std::path::PathBuf::from(dir)).unwrap(), Volumes::default()))
}

/// With the install (`NFSMW_GAME_DIR`) a real car's data drives the mixer: revving raises the Ginsu
/// frequency and the accelerate loop is audible. Without it the test passes without checking anything.
#[test]
fn a_real_car_revs_up_and_is_audible() {
    let Some(mut audio) = audio() else { return };
    let car = audio.load_car_engine("BMWM3GTR").unwrap();
    let mut mixer = EngineMixer::new(&tuning(&car.sound, car.accel_min_frequency()));
    let at = |rpm_pct: f32, throttle: f32| CarInput { rpm_pct, throttle, gear: GEAR_FIRST, ..CarInput::default() };
    let idle = (0..120).map(|_| mixer.update(TICK, &at(0.0, 0.0))).last().unwrap();
    let revving = (0..240).map(|_| mixer.update(TICK, &at(0.8, 1.0))).last().unwrap();
    assert!(revving.ginsu_frequency > idle.ginsu_frequency + 500.0, "{idle:?} -> {revving:?}");
    assert!(revving.accel_volume > 0.1, "{}", revving.accel_volume);
}

/// One scripted second of telemetry: `(rpm fraction, throttle, gear)` at time `t`.
fn script(t: f32) -> (f32, f32, i32) {
    match t {
        t if t < 2.0 => (0.0, 0.0, 2),
        t if t < 5.0 => (0.05 + (t - 2.0) / 3.0 * 0.9, 1.0, 2),
        t if t < 5.5 => (0.95 - (t - 5.0) * 0.6, 0.0, 3),
        t if t < 8.5 => (0.65 + (t - 5.5) / 3.0 * 0.3, 1.0, 3),
        t if t < 9.0 => (0.95 - (t - 8.5) * 0.5, 0.0, 4),
        t => (0.7 - ((t - 9.0) * 0.2).min(0.65), 0.0, 4),
    }
}

fn write_wav(path: &std::path::Path, samples: &[f32]) {
    let data: Vec<u8> = samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend([16u32.to_le_bytes().as_slice(), &1u16.to_le_bytes(), &1u16.to_le_bytes()].concat());
    wav.extend((RATE as u32).to_le_bytes());
    wav.extend((RATE as u32 * 2).to_le_bytes());
    wav.extend([2u16.to_le_bytes().as_slice(), &16u16.to_le_bytes()].concat());
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(data);
    std::fs::write(path, wav).unwrap();
}

/// Renders a drive of the M3 GTR (idle, a pull with two shifts, a lift) through the mixer, the engine voice and
/// the sample layer, as the game does, and checks the level and that the sound moves with the revs. With
/// `NFSMW_ENGINE_WAV` set the sound is written there to listen to.
#[test]
fn a_scripted_drive_sounds_like_an_engine() {
    let Some(mut audio) = audio() else { return };
    let car = audio.load_car_engine("BMWM3GTR").unwrap();
    let mut mixer = EngineMixer::new(&tuning(&car.sound, car.accel_min_frequency()));
    let silent = EngineMix::shared(car.sound.engine.min_rpm, 1.0, 0.0, 0.0);
    let (mut sound, handle) = EngineVoice { start: silent, ..car.voice }.into_sound().unwrap();
    let info = kira::info::MockInfoBuilder::new().build();
    let per_tick = (RATE / 60.0) as usize;
    let mut dynamic = crate::audio::mixer::CarMixer::load(&audio.dir, true).expect("the mixer map loads");
    let mut layer = AemsLayer::load(&mut audio, &car.sound).expect("the sample layer loads");
    let (mut soft, mut layer_out) = (Soft::default(), Vec::new());
    let mut gain = 1.0;
    let (mut out, mut shifts) = (Vec::new(), 0);
    for i in 0..(14.0 * 60.0) as usize {
        let (rpm_pct, throttle, gear) = script(i as f32 * TICK);
        let input = CarInput { rpm_pct, throttle, gear, speed: 10.0 + rpm_pct * 40.0, ..CarInput::default() };
        let o = mixer.update(TICK, &input);
        shifts += usize::from(o.events.gear_clunk.is_some());
        handle.set(EngineMix {
            accel: LoopMix {
                frequency: o.accel_loop.frequency,
                volume: o.accel_volume * gain,
                pitch: o.accel_loop.playback_rate,
            },
            decel: LoopMix {
                frequency: o.decel_loop.frequency,
                volume: o.decel_volume * gain,
                pitch: o.decel_loop.playback_rate,
            },
        });
        let frame = crate::audio::mixer::Frame {
            input: &input,
            engine: &o,
            effects: Default::default(),
            master_volume: car.sound.engine.master_volume,
            sparks: layer.sparks(),
        };
        let levels = dynamic.update(TICK, &frame);
        gain = levels.engine_volume;
        layer.update(
            &mut SoftSampler { audio: &mut audio, mix: &mut soft },
            TICK,
            &Feed { engine: &o, levels: &levels },
        );
        let mut block = vec![Frame::ZERO; per_tick];
        sound.process(&mut block, 1.0 / RATE, &info);
        let mut samples = vec![0.0; per_tick];
        soft.render(&mut samples);
        layer_out.extend(samples.iter().copied());
        out.extend(block.iter().zip(&samples).map(|(f, s)| f.left + s));
    }
    if let Some(path) = std::env::var_os("NFSMW_ENGINE_WAV") {
        write_wav(std::path::Path::new(&path), &out);
    }
    let rms = |from: f32, to: f32| {
        let part = &out[(from * RATE as f32) as usize..(to * RATE as f32) as usize];
        (part.iter().map(|s| s * s).sum::<f32>() / part.len() as f32).sqrt()
    };
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let layer_peak = layer_out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    eprintln!(
        "peak {peak:.3} (sample layer alone {layer_peak:.3}); rms idle {:.4}, pull {:.4}, high revs {:.4}, lift {:.4}; {shifts} gear clunks",
        rms(0.5, 2.0),
        rms(2.5, 5.0),
        rms(4.0, 5.0),
        rms(10.0, 13.0)
    );
    assert!(out.iter().all(|s| s.is_finite()));
    assert!(peak < 1.5, "the engine alone should not clip: {peak}");
    assert!(rms(0.5, 2.0) > 0.005, "the idle is audible: {}", rms(0.5, 2.0));
    assert!(rms(2.5, 5.0) > rms(0.5, 2.0) * 0.5, "the pull is not quieter than half the idle");
}

/// A car shifted by hand (the real physics, gears changed on request) makes the shift sounds: one clunk for each
/// upshift and one for each downshift taken at revs, none when nothing shifts, and neutral and first from standing
/// are silent. The mixer hears the gear number the vehicle reports, as in the game (`SoundFeed`).
#[test]
fn a_hand_shifted_drive_plays_the_shift_sounds() {
    use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle};
    use nfsmw_data::car::physics::PhysicsData;

    let Some(mut audio) = audio() else { return };
    let car = audio.load_car_engine("BMWM3GTR").unwrap();
    let mut mixer = EngineMixer::new(&tuning(&car.sound, car.accel_min_frequency()));
    let mut vehicle = Vehicle::new(PhysicsData::load(&audio.dir).unwrap().car("BMWM3GTR").unwrap().spec);
    vehicle.config.automatic = false;
    let ground = FlatGround::new(0.0);
    assert!(vehicle.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    let (red, idle) = (vehicle.spec().engine.red_line, vehicle.spec().engine.idle);

    let (mut gear_changes, mut clunks) = (Vec::new(), Vec::new());
    let mut previous = vehicle.gear();
    for i in 0..(16.0 * 60.0) as usize {
        let t = i as f32 * FIXED_STEP;
        // Pull away, shift up at 90% of the red line to third, lift and drop to second at 10 s.
        let up = vehicle.rpm() > 0.9 * red && vehicle.gear() < 4 && t < 10.0 && !vehicle.powertrain().shifting();
        let down = i == 10 * 60;
        let throttle = if t < 10.0 { 1.0 } else { 0.0 };
        vehicle.step(
            FIXED_STEP,
            &InputState { throttle, shift_up: up, shift_down: down, ..Default::default() },
            &ground,
        );
        if vehicle.gear() != previous {
            gear_changes.push((previous, vehicle.gear()));
            previous = vehicle.gear();
        }
        let span = red - idle;
        let input = CarInput {
            rpm_pct: ((vehicle.rpm() - idle) / span).clamp(0.0, 1.0),
            throttle,
            gear: vehicle.gear() as i32,
            speed: vehicle.forward_speed().abs(),
            ..CarInput::default()
        };
        clunks.extend(mixer.update(TICK, &input).events.gear_clunk);
    }
    let ups = gear_changes.iter().filter(|(from, to)| to > from).count();
    assert!(ups >= 2, "the pull should take at least two upshifts: {gear_changes:?}");
    assert_eq!(gear_changes.last().map(|g| g.1), Some(vehicle.gear()));
    let up_clunks = clunks.iter().filter(|d| **d == blackbox_carsound::ShiftDirection::Up).count();
    assert_eq!(up_clunks, ups, "one clunk per upshift: {gear_changes:?} against {clunks:?}");
    let downs = gear_changes.iter().filter(|(from, to)| to < from).count();
    assert!(clunks.len() <= ups + downs, "no clunk without a shift: {clunks:?}");
}

/// Every effect of a car with a turbo and one without resolves to a sound its bank really has.
#[test]
fn every_effect_resolves_to_a_sound_in_its_bank() {
    use blackbox_carsound::{ScrapeKind, SoundRef};

    let Some(mut audio) = audio() else { return };
    let mut effects = vec![
        SoundRef::GearClunk { up: true },
        SoundRef::GearClunk { up: false },
        SoundRef::Sweetener(SweetenerKind::Disengage),
        SoundRef::Sweetener(SweetenerKind::Engage),
        SoundRef::ReverseWhine,
        SoundRef::Nitrous,
        SoundRef::Purge,
        SoundRef::Wind,
        SoundRef::Scrape(ScrapeKind::Ground),
        SoundRef::Scrape(ScrapeKind::Wall),
        SoundRef::Scrape(ScrapeKind::Car),
    ];
    effects.extend((0..=6).map(SoundRef::RoadNoise));
    for surface in 0..=1 {
        effects.extend([true, false].map(|sideways| SoundRef::Skid { surface, sideways }));
    }
    let mut turbo = vec![SoundRef::TurboSpool];
    turbo.extend((0..3).map(SoundRef::TurboBlowoff));
    for (name, with_turbo) in [("BMWM3GTR", false), ("SKYLINEZT", true)] {
        let car = audio.load_car_engine(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        let wanted = effects.iter().copied().chain(if with_turbo { turbo.clone() } else { Vec::new() });
        for effect in wanted {
            let target =
                crate::audio::refs::resolve(&car.sound, effect).unwrap_or_else(|| panic!("{name}: {effect:?}"));
            audio.bank_sound(&target.bank, target.index).unwrap_or_else(|e| panic!("{name}: {effect:?}: {e}"));
        }
    }
}
