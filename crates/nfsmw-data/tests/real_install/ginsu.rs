//! The Ginsu synthesiser on the real engine loops: decoded `.gin` files played at a requested frequency
//! come out with the pitch the tables promise.

use std::sync::Arc;

use blackbox_attrib::Database;
use blackbox_ginsu::{FREQUENCY_PER_HZ, GinsuData, GinsuSynth, SynthParams};
use nfsmw_data::sound::{EngineLoops, EngineSound, SoundUpgrades, car_sound, load_loop};

use crate::install;

const BLOCK: usize = 512;

fn render(synth: &mut GinsuSynth, params: &SynthParams, seconds: f32) -> Vec<f32> {
    let mut out = vec![0.0; (seconds * synth.sample_rate() as f32) as usize];
    for block in out.chunks_mut(BLOCK) {
        synth.render(params, block);
    }
    out
}

/// The period in samples of a periodic signal: the lag within `lo ..= hi` with the best normalised
/// autocorrelation, and that correlation.
fn period(signal: &[f32], lo: usize, hi: usize) -> (usize, f32) {
    let energy: f32 = signal.iter().map(|s| s * s).sum();
    let corr: Vec<f32> = (lo..=hi)
        .map(|lag| signal.iter().zip(&signal[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy.max(1e-12))
        .collect();
    let (at, best) = corr.iter().enumerate().fold((0, f32::MIN), |m, (i, &c)| if c > m.1 { (i, c) } else { m });
    (lo + at, best)
}

fn check_steady(data: &Arc<GinsuData>, label: &str) {
    let tables = data.tables();
    let rate = tables.sample_rate() as f32;
    let (min, max) = (tables.min_frequency(), tables.max_frequency());
    for x in [0.25f32, 0.5, 0.75] {
        let f = min + (max - min) * x;
        let mut synth = GinsuSynth::new(data.clone(), min).unwrap();
        let params = SynthParams::new(f);
        let _warm_up = render(&mut synth, &params, 0.6);
        let out = render(&mut synth, &params, 0.6);
        assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 1.0), "{label} at {f}: bad samples");
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 0.01, "{label} at {f}: silent (rms {rms})");

        // The loop repeats one cycle of the recording, so its period is that cycle's length.
        let cycle = tables.sample_to_cycle(tables.frequency_to_sample(f));
        let table_period = tables.cycle_period(cycle + 0.5);
        let nominal = rate * FREQUENCY_PER_HZ / f;
        assert!(
            (table_period / nominal - 1.0).abs() < 0.2,
            "{label} at {f}: the table says {table_period}, nominal {nominal}"
        );
        let (got, strength) = period(&out, (table_period * 0.9) as usize, (table_period * 1.1) as usize);
        let ratio = got as f32 / table_period;
        assert!(
            (0.97..=1.03).contains(&ratio),
            "{label} at {f}: period {got}, the table says {table_period} (x{ratio:.3})"
        );
        assert!(strength > 0.8, "{label} at {f}: weak repetition {strength}");

        // And the synthesiser reports the frequency it is playing.
        let reported = synth.current_frequency();
        assert!((reported / f - 1.0).abs() < 0.08, "{label} at {f}: reports {reported}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_loops_play_at_the_requested_pitch() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let car = car_sound(&db, "BMWM3GTR", SoundUpgrades::default()).unwrap();
    let loops = EngineLoops::load(&dir, &car.engine).unwrap();
    check_steady(&loops.accel.expect("accelerate loop"), "GIN_TVR_Cerbera");
    check_steady(&loops.decel.expect("decelerate loop"), "GIN_TVR_Cerbera_DCL");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_dozen_other_engines_play_at_the_requested_pitch() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let engines: Vec<EngineSound> =
        db.collections_of("engineaudio").map(EngineSound::from_collection).filter(|e| e.has_ginsu()).collect();
    assert!(engines.len() > 60);
    for engine in engines.iter().step_by(6) {
        let data = Arc::new(load_loop(&dir, &engine.accel_loop).unwrap());
        check_steady(&data, &engine.accel_loop);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_pull_through_the_rev_range_glides_without_jumps_in_level() {
    let Some(dir) = install() else { return };
    let data = Arc::new(load_loop(&dir, "GIN_TVR_Cerbera.gin").unwrap());
    let tables = data.tables();
    let (min, max) = (tables.min_frequency(), tables.max_frequency());
    let mut synth = GinsuSynth::new(data.clone(), min).unwrap();
    let seconds = 4.0;
    let rate = synth.sample_rate() as usize;
    let total = (seconds * rate as f32) as usize;
    let mut out = vec![0.0; total];
    let mut reported = Vec::new();
    for (i, block) in out.chunks_mut(BLOCK).enumerate() {
        let x = (i * BLOCK) as f32 / total as f32;
        synth.render(&SynthParams::new(min + (max - min) * x), block);
        reported.push(synth.current_frequency());
    }
    // The reported frequency follows the request upward through the whole range.
    assert!(reported.windows(2).all(|w| w[1] >= w[0] * 0.98), "the reported pitch fell");
    let end = *reported.last().unwrap();
    assert!((end / max - 1.0).abs() < 0.1, "ends at {end}, wanted about {max}");
    // No block-to-block jump in loudness: the level of 50 ms windows stays within a factor of 3.
    let window = rate / 20;
    let rms: Vec<f32> = out
        .chunks(window)
        .filter(|w| w.len() == window)
        .map(|w| (w.iter().map(|s| s * s).sum::<f32>() / window as f32).sqrt())
        .collect();
    for pair in rms.windows(2) {
        assert!(pair[1] < pair[0] * 3.0 && pair[0] < pair[1] * 3.0, "level jumps from {} to {}", pair[0], pair[1]);
    }
    assert!(out.iter().all(|s| s.abs() <= 1.0));
}
