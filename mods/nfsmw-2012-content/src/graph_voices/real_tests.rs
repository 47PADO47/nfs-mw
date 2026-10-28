//! The Porsche's graphs through the mod's builder, against the 2012 install (`MW2012_GAME_DIR`).

use chameleon_bundle::Bundle;

use super::*;
use crate::sample;

#[test]
#[ignore = "needs the 2012 install"]
fn the_porsche_gets_three_ginsu_voices_an_idle_loop_and_five_whine_loops() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let bytes = std::fs::read(std::path::Path::new(&dir).join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let samples = sample::read_all(&bundle);
    let graphs = SoundGraphs::read(&bundle);
    let layers = build(&bundle, &graphs, &samples);
    for layer in &layers {
        let curved = layer.curves.as_ref().unwrap();
        println!("{} {} bytes: {}", if curved.ginsu { "GIN" } else { "WAV" }, layer.wav.len(), curved.fields);
    }
    assert_eq!(layers.iter().filter(|layer| layer.curves.as_ref().is_some_and(|c| c.ginsu)).count(), 3);
    assert_eq!(layers.len(), 9);
    let table: String = layers
        .iter()
        .enumerate()
        .map(|(at, layer)| {
            let curved = layer.curves.as_ref().unwrap();
            let extension = if curved.ginsu { "GIN" } else { "WAV" };
            format!("  {{ file = \"SOUND/MODENGINE/X/L{at}.{extension}\", {} }},\n", curved.fields)
        })
        .collect();
    let text = format!(
        "role = \"player\"\nname = \"X\"\n[engine_sound]\nidle_rpm = 1000.0\nredline_rpm = 7800.0\nlayers = [\n{table}]\n"
    );
    let manifest = blackbox_modding::content::parse_car("x", &text).expect("the game accepts the table");
    assert_eq!(manifest.engine_sound.unwrap().layers.len(), 9);
}

/// The strongest frequency of `samples` between 50 and 3,000 Hz, from a 16,384-sample stretch of the middle.
fn dominant_hz(samples: &[f32], rate: u32) -> f32 {
    let window = 16_384.min(samples.len());
    let from = (samples.len() - window) / 2;
    let stretch = &samples[from..from + window];
    let power = |hz: f32| {
        let step = std::f32::consts::TAU * hz / rate as f32;
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (n, sample) in stretch.iter().enumerate() {
            re += sample * (step * n as f32).cos();
            im += sample * (step * n as f32).sin();
        }
        re * re + im * im
    };
    (25..1500).map(|k| k as f32 * 2.0).max_by(|a, b| power(*a).total_cmp(&power(*b))).unwrap_or(0.0)
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_whine_loops_report_their_strongest_frequency_per_mph_of_the_speed_their_item_names() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let bytes = std::fs::read(std::path::Path::new(&dir).join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let samples = sample::read_all(&bundle);
    let graphs = SoundGraphs::read(&bundle);
    let whine = graphs.driven_by(WHINE_SPEED).next().unwrap();
    let mut ratios = Vec::new();
    for item in &whine.items {
        let ItemKind::Wave(wave) = &item.kind else { continue };
        let sample = samples.iter().find(|sample| wave.waves.first() == Some(&sample.id)).unwrap();
        let hz = dominant_hz(&sample.samples, sample.rate);
        println!(
            "window {}..{} base {} mph: strongest {hz} Hz, {:.2} Hz per mph",
            item.lo,
            item.hi,
            wave.pitch.base,
            hz / wave.pitch.base
        );
        ratios.push(hz / wave.pitch.base);
    }
    // Reported, not asserted: three of the five loops sit between 9 and 13 Hz per mph, two do not (docs/formats/chameleon-sound-graph.md).
    assert_eq!(ratios.len(), 5);
}

use crate::effects;

const TESLA: &str = "VEHICLES/VEH_1085764_HI.BNDL";
/// A hybrid: an engine and an electric motor.
const HYBRID: &str = "VEHICLES/VEH_1085830_HI.BNDL";
/// A turbocharged car, and a high-revving exotic.
const TURBO: &str = "VEHICLES/VEH_1085698_HI.BNDL";
const EXOTIC: &str = "VEHICLES/VEH_1160082_HI.BNDL";

/// The layers the builder gives the car in `file` of the install and the engine table the game parses from them.
fn engine_of(file: &str) -> Option<(Vec<Layer>, blackbox_modding::content::EngineSound, SoundGraphs)> {
    let dir = std::env::var_os("MW2012_GAME_DIR")?;
    let bytes = std::fs::read(std::path::Path::new(&dir).join(file)).unwrap();
    let bundle = Bundle::parse(&bytes).unwrap();
    let samples = sample::read_all(&bundle);
    let graphs = SoundGraphs::read(&bundle);
    let layers = build(&bundle, &graphs, &samples);
    let table: String = layers
        .iter()
        .enumerate()
        .map(|(at, layer)| {
            let curved = layer.curves.as_ref().unwrap();
            let extension = if curved.ginsu { "GIN" } else { "WAV" };
            format!("  {{ file = \"SOUND/MODENGINE/X/L{at}.{extension}\", {} }},\n", curved.fields)
        })
        .collect();
    let text = format!(
        "role = \"player\"\nname = \"X\"\n[engine_sound]\nidle_rpm = 1000.0\nredline_rpm = 7800.0\nlayers = [\n{table}]\n"
    );
    let manifest = blackbox_modding::content::parse_car("x", &text).expect("the game accepts the table");
    Some((layers, manifest.engine_sound.unwrap(), graphs))
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_electric_car_is_two_motor_loops_that_rise_with_the_speed_and_a_quiet_whine() {
    use blackbox_modding::content::EngineInput;

    let Some((_, sound, graphs)) = engine_of(TESLA) else { return };
    assert!(sound.layers.iter().all(|layer| layer.kind == blackbox_modding::content::EngineLayerKind::Loop));
    assert_eq!(sound.layers.len(), 7, "two motor loops and the five loops of the gearbox whine");
    let (motors, whines): (Vec<_>, Vec<_>) = sound.layers.iter().partition(|layer| layer.rate.is_some());
    assert_eq!(motors.len(), 2);
    assert_eq!(whines.len(), 5);
    let mut starts = Vec::new();
    for motor in &motors {
        let rate = motor.rate.as_ref().unwrap();
        assert_eq!(rate.input, EngineInput::SpeedMph);
        assert!(rate.points.windows(2).all(|pair| pair[1].1 >= pair[0].1), "{:?}", rate.points);
        assert_eq!(motor.volume, 2.0, "the graph gives each loop a gain of 2");
        starts.push((rate.points[0].1, rate.points.last().unwrap().1));
        println!("motor rate {:?}", rate.points);
    }
    starts.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!((starts[0].0 - 0.8).abs() < 1e-3 && (starts[0].1 - 1.2).abs() < 1e-3, "{starts:?}");
    assert!((starts[1].0 - 1.0).abs() < 1e-3 && (starts[1].1 - 2.19).abs() < 0.02, "{starts:?}");
    for whine in &whines {
        assert!(whine.pitch.is_some());
        assert!(whine.gain.iter().any(|curve| curve.input == EngineInput::Load), "the gas pedal crossfades the whine");
        assert!(motors[0].volume / whine.volume > 20.0, "the whine is a detail under the motor: {}", whine.volume);
    }
    assert!(effects::has_no_gear_change(&graphs), "an electric car has no gear change sound");
    assert!(graphs.sequences.iter().all(|sequence| !sequence.driven_by("GetPlayerAudibleRPM")));
}

#[test]
#[ignore = "needs the 2012 install"]
fn a_hybrid_has_its_engine_and_a_motor_that_swells_and_fades_with_the_speed() {
    use blackbox_modding::content::{EngineInput, EngineLayerKind};

    let Some((_, sound, graphs)) = engine_of(HYBRID) else { return };
    assert_eq!(sound.layers.iter().filter(|layer| layer.kind == EngineLayerKind::Ginsu).count(), 3);
    let motors: Vec<_> = sound.layers.iter().filter(|layer| layer.rate.is_some()).collect();
    assert_eq!(motors.len(), 1);
    let motor = motors[0];
    let rate = motor.rate.as_ref().unwrap();
    assert!((rate.points[0].1 - 1.8).abs() < 1e-3 && (rate.points.last().unwrap().1 - 2.77).abs() < 0.03);
    // The window starts at 5 mph and fades out over its last 50; the channel's own curve swells to a peak and falls again.
    let swell = motor.gain.iter().find(|curve| curve.points[0] == (5.0, 0.0) && curve.points.iter().any(|p| p.1 > 0.9));
    let peak = swell
        .expect("the mixer item of the motor's channel")
        .points
        .iter()
        .copied()
        .fold((0.0, 0.0), |best, p| if p.1 > best.1 { p } else { best });
    assert!((60.0..=100.0).contains(&peak.0), "the motor peaks mid-speed: {peak:?}");
    assert!(motor.gain.iter().all(|curve| curve.input == EngineInput::SpeedMph));
    assert!(!effects::has_no_gear_change(&graphs), "a hybrid shifts");
}

#[test]
#[ignore = "needs the 2012 install"]
fn combustion_engines_of_different_kinds_keep_three_ginsu_voices_over_their_own_range() {
    use blackbox_modding::content::EngineLayerKind;

    for (file, low, high) in [(TURBO, 2350.0, 7840.0), (EXOTIC, 1800.0, 10000.0)] {
        let Some((_, sound, graphs)) = engine_of(file) else { return };
        let ginsu: Vec<_> = sound.layers.iter().filter(|layer| layer.kind == EngineLayerKind::Ginsu).collect();
        assert_eq!(ginsu.len(), 3, "{file}");
        assert!(ginsu.iter().all(|layer| layer.span == Some((low, high))), "{file}: {:?}", ginsu[0].span);
        assert!(sound.layers.iter().all(|layer| layer.rate.is_none()), "{file}: no motor in a combustion car");
        assert!(!effects::has_no_gear_change(&graphs), "{file}: it shifts");
        let whines = sound.layers.iter().filter(|layer| layer.pitch.is_some()).count();
        assert_eq!(whines, 5, "{file}: the gearbox whine");
    }
}
