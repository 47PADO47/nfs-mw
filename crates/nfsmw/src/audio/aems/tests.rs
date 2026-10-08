//! Against the install: the real banks run on a scripted drive and the voices they ask for are mixed in software
//! (needs `NFSMW_GAME_DIR`; passes without checking otherwise).

use blackbox_carsound::{CarInput, EngineMixer};
use game_install::GameDir;

use super::soft::{RATE, Soft, SoftSampler};
use super::*;
use crate::audio::Volumes;
use crate::audio::mixer::{CarMixer, Frame};
use crate::audio::tuning::tuning;

const TICK: f32 = 1.0 / 60.0;

fn audio() -> Option<Audio> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(Audio::new(GameDir::open(std::path::PathBuf::from(dir)).unwrap(), Volumes::default()))
}

/// One scripted drive: `(rpm fraction, throttle, gear)` at time `t`. Idle, a pull to the redline, a lift, a second
/// pull in the next gear, a lift and a coast.
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

/// What a drive of the sample layer measured.
struct Drive {
    /// The layer's output, mono.
    samples: Vec<f32>,
    /// Most voices the engine module and the sputter had at once.
    max_engine: usize,
    max_sputter: usize,
    /// Ticks in which the sputter reported a volume (the spark chatter's mixer input).
    spark_ticks: usize,
    /// The engine module's voices at the end of the idle, long after the first of them started.
    idle_voices: usize,
}

impl Drive {
    fn rms(&self, from: f32, to: f32) -> f32 {
        let part = &self.samples[(from * RATE as f32) as usize..(to * RATE as f32) as usize];
        (part.iter().map(|s| s * s).sum::<f32>() / part.len() as f32).sqrt()
    }

    fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0, |m, s| m.max(s.abs()))
    }
}

/// Drives the layer of `car` through the script, `seconds` long.
fn drive(audio: &mut Audio, car: &str, seconds: f32) -> Drive {
    let loaded = audio.load_car_engine(car).unwrap();
    let mut engine = EngineMixer::new(&tuning(&loaded.sound, loaded.accel_min_frequency()));
    let mut dynamic = CarMixer::load(&audio.dir, true).expect("the mixer map loads");
    let mut layer = AemsLayer::load(audio, &loaded.sound).expect("the layer loads");
    let (mut soft, per_tick) = (Soft::default(), (RATE / 60.0) as usize);
    let mut run = Drive { samples: Vec::new(), max_engine: 0, max_sputter: 0, spark_ticks: 0, idle_voices: 0 };
    for i in 0..(seconds * 60.0) as usize {
        let (rpm_pct, throttle, gear) = script(i as f32 * TICK);
        let input = CarInput { rpm_pct, throttle, gear, speed: 10.0 + rpm_pct * 40.0, ..CarInput::default() };
        let out = engine.update(TICK, &input);
        let frame = Frame {
            input: &input,
            engine: &out,
            effects: Default::default(),
            master_volume: loaded.sound.engine.master_volume,
            sparks: layer.sparks(),
        };
        let levels = dynamic.update(TICK, &frame);
        layer.update(&mut SoftSampler { audio, mix: &mut soft }, TICK, &Feed { engine: &out, levels: &levels });
        let (e, s) = layer.voices();
        (run.max_engine, run.max_sputter) = (run.max_engine.max(e), run.max_sputter.max(s));
        run.spark_ticks += usize::from(layer.sparks());
        if i == 119 {
            run.idle_voices = e;
        }
        let mut block = vec![0.0; per_tick];
        soft.render(&mut block);
        run.samples.extend(block);
    }
    run
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

/// The M3's sample layer over the scripted drive: voices start, hold at most the module's eight players, are
/// audible and stay below clipping. With `NFSMW_AEMS_WAV` set the layer alone is written there to listen to.
#[test]
fn the_sample_layer_plays_over_a_drive() {
    let Some(mut audio) = audio() else { return };
    let run = drive(&mut audio, "BMWM3GTR", 14.0);
    if let Some(path) = std::env::var_os("NFSMW_AEMS_WAV") {
        write_wav(std::path::Path::new(&path), &run.samples);
    }
    eprintln!(
        "sample layer: peak {:.3}; rms idle {:.4}, pull {:.4}, high revs {:.4}, lift {:.4}; voices up to {} + {}; sparks {} ticks",
        run.peak(),
        run.rms(0.5, 2.0),
        run.rms(2.5, 5.0),
        run.rms(4.0, 5.0),
        run.rms(10.0, 13.0),
        run.max_engine,
        run.max_sputter,
        run.spark_ticks
    );
    assert!(run.samples.iter().all(|s| s.is_finite()));
    assert!(run.peak() < 1.0, "the sample layer alone should not clip: {}", run.peak());
    assert!(run.rms(0.5, 2.0) > 0.0005, "the idle has a body: {}", run.rms(0.5, 2.0));
    assert!(run.rms(2.5, 5.0) > 0.0005, "and the pull: {}", run.rms(2.5, 5.0));
    assert!((2..=8).contains(&run.max_engine), "{} engine voices", run.max_engine);
    assert!(run.idle_voices >= 1, "the idle sounds are loops that keep playing, not one-shots");
}

/// Lifting off the throttle at high revs makes the sputter pop: its module starts voices and reports a volume.
#[test]
fn lifting_off_makes_the_sputter_pop() {
    let Some(mut audio) = audio() else { return };
    let run = drive(&mut audio, "BMWM3GTR", 14.0);
    assert!(run.max_sputter > 0, "the sputter started no voice");
    assert!(run.spark_ticks > 0, "the sputter's output object never reported a volume");
}
