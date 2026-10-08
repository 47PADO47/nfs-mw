//! The synthesiser: pitch, glide, joins, determinism and robustness.

use std::sync::Arc;

use super::chirp::{AMPLITUDE, Chirp, chirp, max_step, measure_hz, standard};
use crate::{Error, FREQUENCY_PER_HZ, GinsuData, GinsuSynth, GinsuTables, SynthParams};

const BLOCK: usize = 512;

fn render_seconds(synth: &mut GinsuSynth, params: &SynthParams, seconds: f32) -> Vec<f32> {
    let total = (seconds * synth.sample_rate() as f32) as usize;
    let mut out = vec![0.0; total];
    for block in out.chunks_mut(BLOCK) {
        synth.render(params, block);
    }
    out
}

fn synth_at(chirp: &Chirp, x: f32) -> GinsuSynth {
    GinsuSynth::new(chirp.data.clone(), chirp.freq(x)).unwrap()
}

#[test]
fn a_steady_target_plays_at_that_pitch() {
    let chirp = standard();
    for x in [0.2, 0.35, 0.5, 0.65, 0.8] {
        let f = chirp.freq(x);
        let mut synth = synth_at(&chirp, x);
        let params = SynthParams::new(f);
        let _warm_up = render_seconds(&mut synth, &params, 0.3);
        let out = render_seconds(&mut synth, &params, 1.0);
        let hz = measure_hz(&out, chirp.sample_rate);
        let want = f / FREQUENCY_PER_HZ;
        assert!((hz / want - 1.0).abs() < 0.03, "x {x}: {hz} Hz, wanted {want} Hz");
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > AMPLITUDE * 0.9 && peak < AMPLITUDE * 1.05, "x {x}: peak {peak}");
    }
}

#[test]
fn a_target_far_from_the_start_is_reached() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 0.1);
    let f = chirp.freq(0.9);
    let params = SynthParams::new(f);
    let _ = render_seconds(&mut synth, &params, 0.8);
    let out = render_seconds(&mut synth, &params, 1.0);
    let hz = measure_hz(&out, chirp.sample_rate);
    assert!((hz / (f / FREQUENCY_PER_HZ) - 1.0).abs() < 0.03, "{hz} Hz");
    let reported = synth.current_frequency();
    assert!((reported / f - 1.0).abs() < 0.05, "reports {reported}, wanted {f}");
    assert!((synth.current_pitch() * FREQUENCY_PER_HZ - reported).abs() < 1e-3);
}

#[test]
fn the_pitch_glides_up_with_a_rising_target() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 0.2);
    let seconds = 3.0;
    let rate = chirp.sample_rate as usize;
    let total = (seconds * rate as f32) as usize;
    let mut out = vec![0.0; total];
    for (i, block) in out.chunks_mut(BLOCK).enumerate() {
        let x = 0.2 + 0.6 * (i * BLOCK) as f32 / total as f32;
        synth.render(&SynthParams::new(chirp.freq(x.min(0.8))), block);
    }
    assert!(out.iter().all(|s| s.is_finite()));
    let window = rate / 4;
    let mut last = 0.0;
    for (n, w) in out[rate / 4..].chunks(window).take_while(|w| w.len() == window).enumerate() {
        let hz = measure_hz(w, chirp.sample_rate);
        assert!(hz > last * 0.97, "window {n}: {hz} Hz after {last} Hz");
        last = hz;
    }
    let want = chirp.freq(0.8) / FREQUENCY_PER_HZ;
    assert!((last / want - 1.0).abs() < 0.08, "ends at {last} Hz, wanted {want} Hz");
}

#[test]
fn jumps_between_cycles_leave_no_clicks() {
    let chirp = standard();
    let hz_max = (chirp.hz0 + (chirp.hz1 - chirp.hz0) * 0.85) as f32;
    let bound = AMPLITUDE * std::f32::consts::TAU * hz_max / chirp.sample_rate as f32 * 1.5;
    // Steady targets at several spots: each loops one cycle and joins it to itself many times.
    for x in [0.15, 0.4, 0.7, 0.85] {
        let mut synth = synth_at(&chirp, x);
        let out = render_seconds(&mut synth, &SynthParams::new(chirp.freq(x)), 1.5);
        assert!(max_step(&out) < bound, "x {x}: step {} over {bound}", max_step(&out));
    }
    // A sweep up and down.
    let mut synth = synth_at(&chirp, 0.2);
    let mut out = vec![0.0; chirp.sample_rate as usize * 4];
    for (i, block) in out.chunks_mut(BLOCK).enumerate() {
        let phase = i as f32 * 0.04;
        let x = 0.5 + 0.35 * phase.sin();
        synth.render(&SynthParams::new(chirp.freq(x)), block);
    }
    assert!(max_step(&out) < bound, "sweep: step {} over {bound}", max_step(&out));
}

#[test]
fn the_same_calls_give_the_same_samples() {
    let chirp = standard();
    let run = |blocks: &[usize]| {
        let mut synth = synth_at(&chirp, 0.3);
        let params = SynthParams::new(chirp.freq(0.6));
        let mut out = Vec::new();
        for &n in blocks {
            let mut block = vec![0.0; n];
            synth.render(&params, &mut block);
            out.extend(block);
        }
        out
    };
    let a = run(&[8192]);
    assert_eq!(a, run(&[8192]), "same call sequence");
    let pieces = [1, 99, 333, 1000, 7, 2048, 1, 4703];
    assert_eq!(pieces.iter().sum::<usize>(), 8192);
    assert_eq!(a, run(&pieces), "block sizes do not change a steady target's samples");
}

#[test]
fn a_clone_continues_identically() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 0.3);
    let params = SynthParams::new(chirp.freq(0.7));
    let _ = render_seconds(&mut synth, &params, 0.2);
    let mut copy = synth.clone();
    let a = render_seconds(&mut synth, &params, 0.3);
    let b = render_seconds(&mut copy, &params, 0.3);
    assert_eq!(a, b);
}

#[test]
fn a_lower_latency_reaches_a_new_target_sooner() {
    let chirp = standard();
    let from = 0.2;
    let target = chirp.freq(0.8);
    let probe = |latency_ms: f32| {
        let mut synth = synth_at(&chirp, from);
        let _ = render_seconds(&mut synth, &SynthParams::new(chirp.freq(from)), 0.2);
        let params = SynthParams { frequency: target, volume: 1.0, latency_ms };
        let _ = render_seconds(&mut synth, &params, 0.12);
        synth.current_frequency()
    };
    let (quick, slow) = (probe(0.0), probe(1000.0));
    assert!(quick > slow, "{quick} vs {slow}");
    assert!((quick / target - 1.0).abs() < 0.05, "{quick} should be at {target}");
}

#[test]
fn update_frequency_restarts_the_glide() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 0.2);
    let target = chirp.freq(0.8);
    synth.update_frequency(target, 0.0);
    let mut block = vec![0.0; 600];
    // A request that is not a number is ignored, so the target set by hand holds.
    synth.render(&SynthParams::new(f32::NAN), &mut block);
    assert!(synth.current_frequency() > chirp.freq(0.5), "{}", synth.current_frequency());
    synth.update_frequency(f32::NAN, 10.0);
    synth.update_frequency(f32::INFINITY, 10.0);
    let mut more = vec![0.0; 6000];
    synth.render(&SynthParams::new(f32::NAN), &mut more);
    assert!((synth.current_frequency() / target - 1.0).abs() < 0.05);
}

#[test]
fn volume_scales_and_ramps() {
    let chirp = standard();
    let f = chirp.freq(0.5);
    let render = |volumes: &[f32]| {
        let mut synth = synth_at(&chirp, 0.5);
        volumes
            .iter()
            .flat_map(|&v| {
                let mut block = vec![0.0; BLOCK];
                synth.render(&SynthParams::new(f).with_volume(v), &mut block);
                block
            })
            .collect::<Vec<f32>>()
    };
    let full = render(&[1.0, 1.0]);
    let half = render(&[0.5, 0.5]);
    for (a, b) in full.iter().zip(&half) {
        assert!((a * 0.5 - b).abs() < 1e-6);
    }
    assert!(render(&[0.0, 0.0]).iter().all(|&s| s == 0.0));
    let ramp = render(&[1.0, 0.0]);
    assert_eq!(ramp[BLOCK + BLOCK - 1], 0.0, "the ramp ends at silence");
    assert_eq!(ramp[..BLOCK], full[..BLOCK], "the first block is at the first volume");
    let tail: f32 = ramp[BLOCK..].iter().map(|s| s.abs()).sum();
    let head: f32 = full[BLOCK..].iter().map(|s| s.abs()).sum();
    assert!(tail < head * 0.6, "{tail} against {head}");
}

#[test]
fn odd_inputs_never_give_odd_samples() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 0.5);
    let hz = [f32::NAN, f32::INFINITY, -1e9, 0.0, 1.0, 1e9, chirp.freq(0.5), chirp.freq(1.0), chirp.freq(0.0)];
    let volumes = [f32::NAN, -3.0, 0.0, 2.0, f32::INFINITY, 1.0];
    let mut seed = 12345u32;
    let mut next = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 16) as usize
    };
    for _ in 0..400 {
        let params = SynthParams {
            frequency: hz[next() % hz.len()],
            volume: volumes[next() % volumes.len()],
            latency_ms: (next() % 300) as f32 - 20.0,
        };
        let mut block = vec![0.0; next() % 900];
        synth.render(&params, &mut block);
        assert!(block.iter().all(|s| s.is_finite() && s.abs() <= 2.5), "{params:?}");
    }
    let mut empty = [];
    synth.render(&SynthParams::new(1.0), &mut empty);
}

#[test]
fn the_end_of_the_recording_plays_without_a_hole() {
    let chirp = standard();
    let mut synth = synth_at(&chirp, 1.0);
    let params = SynthParams::new(chirp.freq(1.0) * 2.0);
    let out = render_seconds(&mut synth, &params, 1.5);
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(rms > AMPLITUDE * 0.5, "rms {rms}");
    let mut synth = synth_at(&chirp, 0.0);
    let out = render_seconds(&mut synth, &SynthParams::new(0.0), 1.5);
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(rms > AMPLITUDE * 0.5, "rms {rms} at the bottom");
}

#[test]
fn other_sample_rates_work() {
    for rate in [22_050, 24_000, 36_000, 44_100] {
        let chirp = chirp(rate, 4.0, 12.0, 50.0);
        let f = chirp.freq(0.5);
        let mut synth = synth_at(&chirp, 0.5);
        let _ = render_seconds(&mut synth, &SynthParams::new(f), 0.3);
        let out = render_seconds(&mut synth, &SynthParams::new(f), 1.0);
        let hz = measure_hz(&out, rate);
        assert!((hz / (f / FREQUENCY_PER_HZ) - 1.0).abs() < 0.03, "{rate}: {hz} Hz");
    }
}

#[test]
fn unusable_data_is_refused() {
    let tables = |rate, cycles: Vec<u32>| GinsuTables::new(1.0, 2.0, rate, 100, vec![0, 10], cycles).unwrap();
    let data = |t: GinsuTables| Arc::new(GinsuData::new(t, vec![0.0; 100]).unwrap());
    assert!(matches!(GinsuSynth::new(data(tables(1000, vec![0, 50, 99])), 1.5), Err(Error::SampleRateTooLow(1000))));
    assert!(matches!(GinsuSynth::new(data(tables(24_000, vec![0])), 1.5), Err(Error::NoCycles)));
    assert!(GinsuSynth::new(data(tables(24_000, vec![0, 50, 99])), 1.5).is_ok());
    let t = tables(24_000, vec![0, 50, 99]);
    assert!(matches!(
        GinsuData::new(t.clone(), vec![0.0; 99]),
        Err(Error::SampleCountMismatch { expected: 100, actual: 99 })
    ));
    let from_i16 = GinsuData::from_i16(t, &[16384; 100]).unwrap();
    assert_eq!(from_i16.pcm()[0], 0.5);
}

#[test]
fn reads_outside_the_recording_are_silent() {
    let chirp = standard();
    let mut buf = [1.0f32; 6];
    chirp.data.read(-3, &mut buf);
    assert_eq!(&buf[..3], &[0.0; 3]);
    assert_eq!(buf[3], chirp.data.pcm()[0]);
    let end = chirp.data.pcm().len() as i32;
    let mut buf = [1.0f32; 4];
    chirp.data.read(end - 2, &mut buf);
    assert_eq!(buf[0], chirp.data.pcm()[end as usize - 2]);
    assert_eq!(&buf[2..], &[0.0, 0.0]);
    chirp.data.read(i32::MAX, &mut buf);
    assert_eq!(buf, [0.0; 4]);
}
