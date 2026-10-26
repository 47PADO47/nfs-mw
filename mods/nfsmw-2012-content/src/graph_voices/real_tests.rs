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
