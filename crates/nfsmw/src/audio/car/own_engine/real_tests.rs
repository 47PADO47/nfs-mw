//! A converted 2012 car's own engine through the real mixer, headless. Ignored without `NFSMW_GAME_DIR` (the 2005
//! install) and `NFSMW_MOD_DIR` (a mod folder holding converted cars, such as the one `nfsmw-2012-content` writes).

use std::path::PathBuf;

use game_install::GameDir;
use nfsmw_data::car::mods::{self as car_mods, CurveInput, EngineLayer, EngineSound, LayerCurve, LayerPitch};

use super::*;
use crate::modding::content::{ModDir, rescan_in};

const RATE: u32 = 48_000;

fn left(stereo: &[f32]) -> Vec<f32> {
    stereo.as_chunks::<2>().0.iter().map(|frame| frame[0]).collect()
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

/// The fundamental of `samples` in Hz: the first strong peak of the normalised autocorrelation between 20 and 800 Hz.
fn pitch(samples: &[f32]) -> f32 {
    let window = 16_384.min(samples.len() / 2);
    let stretch = &samples[samples.len() - window..];
    let energy: f32 = stretch.iter().map(|s| s * s).sum();
    let (shortest, longest) = (RATE as usize / 800, (RATE as usize / 20).min(window / 2));
    let correlation: Vec<f32> = (shortest..longest)
        .map(|lag| stretch[..window - lag].iter().zip(&stretch[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy)
        .collect();
    let best = correlation.iter().copied().fold(f32::MIN, f32::max);
    let at = (1..correlation.len() - 1)
        .find(|&at| {
            correlation[at] >= 0.85 * best
                && correlation[at] >= correlation[at - 1]
                && correlation[at] > correlation[at + 1]
        })
        .expect("a peak");
    RATE as f32 / (shortest + at) as f32
}

/// The sound of `engine` at `fraction`, one second after it is set, as the left channel.
fn render(audio: &mut Audio, engine: &mut OwnEngine, fraction: f32) -> Vec<f32> {
    engine.update(fraction, 1.0);
    audio.render_replay(24_000);
    left(audio.render_replay(RATE as usize))
}

#[test]
#[ignore = "reads the 2005 install and a converted mod folder (NFSMW_GAME_DIR, NFSMW_MOD_DIR)"]
fn the_cars_own_engine_plays_at_every_speed_and_follows_the_rpm_in_the_real_mixer() {
    let (Some(game), Some(mod_dir)) = (std::env::var_os("NFSMW_GAME_DIR"), std::env::var_os("NFSMW_MOD_DIR")) else {
        return;
    };
    let dir = GameDir::open(PathBuf::from(game)).unwrap();
    let id = "nfsmw-2012-content";
    rescan_in(&dir, &[ModDir { id: id.into(), dir: PathBuf::from(mod_dir), priority: 0 }], id).unwrap();
    let car = car_mods::published().iter().find(|car| car.folder == "POR911CARR12").expect("the Porsche is converted");
    let sound = car_mods::engine_sound(&car.folder).expect("the Porsche has engine loops");
    println!("{}: {} layers, idle {} redline {}", car.folder, sound.layers.len(), sound.idle_rpm, sound.redline_rpm);

    // The car has no record in the sound database: its set is the class-wide defaults, named by the car's folder.
    let database = nfsmw_data::shared::attributes(&dir).unwrap();
    assert!(nfsmw_data::sound::car_sound(&database, "por911carr12", Default::default()).is_err());
    let set = nfsmw_data::sound::generic_car_sound(&database, "por911carr12").expect("the default sound set");
    assert_eq!(set.car, "por911carr12");

    // Every speed of the car's whole engine is audible.
    let mut audio = Audio::replay(dir, RATE);
    let mut engine = audio.start_own_engine(sound).expect("the loops start");
    for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let level = rms(&render(&mut audio, &mut engine, fraction));
        println!("  {fraction:.2} of the way to redline ({:.0} rpm): rms {level:.3}", sound.rpm_at(fraction));
        assert!(level > 0.01, "silent at {fraction}");
    }
    drop(engine);

    // One layer alone is sped up with the rpm: at 1.5 times its own rpm it sounds one and a half times higher.
    let layer = &sound.layers[0];
    let alone = Box::leak(Box::new(EngineSound {
        idle_rpm: layer.rpm,
        redline_rpm: layer.rpm * 1.5,
        layers: vec![layer.clone()],
    }));
    let mut engine = audio.start_own_engine(alone).expect("the one layer starts");
    let (natural, raised) =
        (pitch(&render(&mut audio, &mut engine, 0.0)), pitch(&render(&mut audio, &mut engine, 1.0)));
    println!("  one layer: {natural:.1} Hz at its own rpm, {raised:.1} Hz at 1.5 times it");
    assert!((raised / natural - 1.5).abs() < 0.1, "{natural} Hz became {raised} Hz");
}

#[test]
#[ignore = "reads the 2005 install (NFSMW_GAME_DIR)"]
fn the_built_in_engine_is_audible_and_rises_with_the_rpm_in_the_real_mixer() {
    let Some(game) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(PathBuf::from(game)).unwrap();
    let mut audio = Audio::replay(dir, RATE);
    let mut engine = audio.start_built_in_engine().expect("the built-in loops start");
    let mut pitches = Vec::new();
    for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let samples = render(&mut audio, &mut engine, fraction);
        let level = rms(&samples);
        println!("  {fraction:.2} of the way to redline: rms {level:.3}, pitch {:.1} Hz", pitch(&samples));
        assert!(level > 0.01, "silent at {fraction}");
        pitches.push(pitch(&samples));
    }
    assert!(pitches.windows(2).all(|pair| pair[1] > pair[0] * 1.1), "the pitch follows the rpm: {pitches:?}");
}

/// The sound of `engine` for `inputs`, one second after they are set, as the left channel.
fn render_inputs(audio: &mut Audio, engine: &mut OwnEngine, inputs: &EngineInputs) -> Vec<f32> {
    engine.update_inputs(inputs, 1.0);
    audio.render_replay(24_000);
    left(audio.render_replay(RATE as usize))
}

fn curve(input: CurveInput, points: &[(f32, f32)]) -> LayerCurve {
    LayerCurve { input, points: points.to_vec() }
}

/// An engine of three loops that play by curves like the decoded ones of a 2012 car: an idle loop that is gone 10 percent
/// above idle, a loop that needs revs and throttle, and a whine that follows the car's speed.
fn curved_engine() -> EngineSound {
    let layer = |file: &str, gain: Vec<LayerCurve>| EngineLayer { gain, ..EngineLayer::classic(file, 0.0) };
    let mut whine =
        layer("whine", vec![curve(CurveInput::SpeedMph, &[(20.0, 0.0), (40.0, 1.0), (90.0, 1.0), (110.0, 0.0)])]);
    whine.pitch = Some(LayerPitch { input: CurveInput::SpeedMph, reference: 40.0 });
    EngineSound {
        idle_rpm: 900.0,
        redline_rpm: 7000.0,
        layers: vec![
            layer("idle", vec![curve(CurveInput::Rpm, &[(0.0, 1.0), (0.1, 0.0)])]),
            layer(
                "revs",
                vec![
                    curve(CurveInput::Rpm, &[(0.0, 0.0), (0.2, 1.0), (1.0, 1.0)]),
                    curve(CurveInput::Load, &[(0.0, 0.0), (1.0, 1.0)]),
                ],
            ),
            whine,
        ],
    }
}

#[test]
#[ignore = "reads the 2005 install (NFSMW_GAME_DIR)"]
fn curves_decide_which_layers_are_heard_in_the_real_mixer() {
    let Some(game) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(PathBuf::from(game)).unwrap();
    let mut audio = Audio::replay(dir, RATE);
    let sources = vec![
        Source::Loop(built_in_loop(900.0)),
        Source::Loop(built_in_loop(4400.0)),
        Source::Loop(built_in_loop(6500.0)),
    ];
    let mut engine = audio.start_layers(curved_engine(), sources).expect("the layers start");
    let mut level = |rpm: f32, load: f32, speed_mph: f32| {
        let level = rms(&render_inputs(&mut audio, &mut engine, &EngineInputs { rpm, load, speed_mph }));
        println!("  rpm {rpm} load {load} speed {speed_mph} mph: rms {level:.3}");
        level
    };
    assert!(level(0.0, 0.0, 0.0) > 0.01, "the idle loop plays at idle");
    assert!(level(1.0, 0.0, 0.0) < 0.002, "at the limiter with the throttle shut nothing is left");
    assert!(level(1.0, 1.0, 0.0) > 0.01, "the revs loop needs both");
    assert!(level(0.5, 0.0, 60.0) > 0.01, "the whine plays at speed with no throttle");
    assert!(level(0.5, 0.0, 130.0) < 0.002, "and is gone above its window");
}
