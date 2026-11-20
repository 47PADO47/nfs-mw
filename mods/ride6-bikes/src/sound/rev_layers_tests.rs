use super::*;
use crate::sound::rev;
use crate::sound::rev::tests::{model, usual};

fn plan(loops: usize) -> Plan {
    Plan { loops, idle_rpm: 2400.0, redline_rpm: 11_000.0 }
}

/// The speeds of the layers under load (`off` false) or off load, slowest first.
fn rpms(layers: &EngineLayers, off: bool) -> Vec<f32> {
    layers.layers.iter().filter(|layer| layer.off_load == off).map(|layer| layer.rpm).collect()
}

/// Recordings that are a sine of a pitch chosen by the stream number, at 48 kHz.
struct Tones;

impl Recordings for Tones {
    fn decode(&self, number: usize) -> Result<Pcm> {
        // Streams 1 to 3 and 5 to 7 are loops (30, 60, 90 Hz and 35, 55, 80 Hz), 9 is the idle (20 Hz, 1.5 s).
        let (hz, frames) = match number {
            1 => (30.0, 28_800),
            2 => (60.0, 28_800),
            3 => (90.0, 28_800),
            5 => (35.0, 28_800),
            6 => (55.0, 28_800),
            7 => (80.0, 28_800),
            9 => (20.0, 72_000),
            _ => (10.0, 1_000),
        };
        let samples = (0..frames)
            .map(|i| (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin() * 0.25 * (1.0 + number as f32 / 20.0))
            .collect();
        Ok(Pcm { sample_rate: 48_000, channels: 1, samples, loop_range: None })
    }
}

/// Loops that are a cosine of exactly the whole number of cycles the table gives for their stream, 0.6 s long.
struct Cycles(&'static [(usize, u32)]);

impl Recordings for Cycles {
    fn decode(&self, number: usize) -> Result<Pcm> {
        let Some(&(_, cycles)) = self.0.iter().find(|(stream, _)| *stream == number) else {
            return Tones.decode(number);
        };
        let samples =
            (0..28_800).map(|i| (std::f32::consts::TAU * cycles as f32 * i as f32 / 28_800.0).cos() * 0.25).collect();
        Ok(Pcm { sample_rate: 48_000, channels: 1, samples, loop_range: None })
    }
}

/// The size of the Fourier bin `k` of `samples` (a cosine of amplitude 1 on a bin is 0.5).
fn bin(samples: &[f32], k: usize) -> f32 {
    let n = samples.len() as f32;
    let (re, im) = samples.iter().enumerate().fold((0.0_f32, 0.0_f32), |(re, im), (i, x)| {
        let angle = std::f32::consts::TAU * ((k * i) % samples.len()) as f32 / n;
        (re + x * angle.cos(), im - x * angle.sin())
    });
    re.hypot(im) / n
}

#[test]
fn indices_spread_over_a_list_with_both_ends() {
    assert_eq!(spread(20, 11), [0, 2, 4, 6, 8, 10, 11, 13, 15, 17, 19]);
    assert_eq!(spread(3, 3), [0, 1, 2]);
    assert_eq!(spread(3, 8), [0, 1, 2]);
    assert_eq!(spread(5, 1), [0]);
    assert!(spread(0, 4).is_empty());
    assert_eq!(spread(19, 11).first(), Some(&0));
    assert_eq!(spread(19, 11).last(), Some(&18));
}

#[test]
fn layers_are_the_loops_under_load_and_off_load_and_the_idle_for_each_at_their_engine_speeds() {
    let parsed = rev::parse(&model(&usual())).unwrap();
    let mut layers = build(&parsed, &Tones, &parsed, &Tones, &plan(3)).unwrap();
    // Under load: the idle (20 Hz, 2400 rpm), then 30, 60 and 90 Hz of the loops: 3600, 7200 and 10800 rpm.
    let on = rpms(&layers, false);
    assert_eq!(on.len(), 4);
    assert!((on[0] - 2400.0).abs() < 60.0, "{on:?}");
    assert!((on[1] - 3600.0).abs() < 0.1 && (on[2] - 7200.0).abs() < 0.1 && (on[3] - 10800.0).abs() < 0.1);
    // Off load: the same idle, then 35, 55 and 80 Hz: 4200, 6600 and 9600 rpm.
    let off = rpms(&layers, true);
    assert_eq!(off.len(), 4);
    assert!((off[0] - on[0]).abs() < 1e-3, "the idle plays in both sets: {off:?}");
    assert!((off[1] - 4200.0).abs() < 0.1 && (off[2] - 6600.0).abs() < 0.1 && (off[3] - 9600.0).abs() < 0.1);
    assert_eq!(layers.worst_detune, 0.0);
    assert!(layers.layers.windows(2).all(|pair| pair[0].rpm <= pair[1].rpm), "slowest first");
    // Engine and exhaust of one number of cycles are summed: twice one tone.
    let loop_frames = layers.layers.iter().find(|layer| !layer.off_load && layer.rpm > 3000.0).unwrap().pcm.frames();
    assert_eq!(loop_frames, 28_800);
    equalise(&mut layers, 0.9).unwrap();
    let peak = layers.layers.iter().map(|layer| layer.pcm.peak()).fold(0.0_f32, f32::max);
    assert!((peak - 0.9).abs() < 1e-3);
    // Every layer has the same loudness.
    let rms: Vec<f32> = layers
        .layers
        .iter()
        .map(|layer| (layer.pcm.samples.iter().map(|s| s * s).sum::<f32>() / layer.pcm.samples.len() as f32).sqrt())
        .collect();
    assert!(rms.iter().all(|r| (r - rms[0]).abs() < 1e-3), "{rms:?}");
    assert_eq!(encode(&layers.layers[1])[..4], *b"RIFF");
}

#[test]
fn an_exhaust_with_other_cycle_counts_is_matched_to_the_nearest_and_the_detune_is_reported() {
    let engine = rev::parse(&model(&usual())).unwrap();
    let mut other = usual();
    other[0].1 = 17;
    other[1].1 = 40;
    let exhaust = rev::parse(&model(&other)).unwrap();
    let layers = build(&engine, &Tones, &exhaust, &Tones, &plan(3)).unwrap();
    // 18 against 17 cycles (5.6%) is summed; 36 against 40 (11%) is left out, and 54 against 54 is a match.
    assert!(layers.worst_detune > 0.05 && layers.worst_detune <= MAX_DETUNE, "{}", layers.worst_detune);
    assert!(layers.notes.iter().any(|note| note.contains("without the exhaust")));
}

#[test]
fn the_exhaust_is_summed_at_the_pitch_of_the_engine_loop_not_at_its_own() {
    const ENGINE: &[(usize, u32)] = &[(1, 18), (2, 36), (3, 54), (5, 21), (6, 33), (7, 48)];
    const EXHAUST: &[(usize, u32)] = &[(1, 17), (2, 40), (3, 54), (5, 21), (6, 33), (7, 48)];
    let engine = rev::parse(&model(&usual())).unwrap();
    let mut other = usual();
    other[0].1 = 17;
    other[1].1 = 40;
    let exhaust = rev::parse(&model(&other)).unwrap();
    let layers = build(&engine, &Cycles(ENGINE), &exhaust, &Cycles(EXHAUST), &plan(3)).unwrap();
    let first = layers.layers.iter().find(|layer| !layer.off_load && (layer.rpm - 3600.0).abs() < 1.0).unwrap();
    // The engine has 18 cycles in the loop and the exhaust 17: summed at the engine's pitch, all the energy is in bin 18.
    assert!((bin(&first.pcm.samples, 18) - 0.25).abs() < 0.01, "bin 18: {}", bin(&first.pcm.samples, 18));
    assert!(bin(&first.pcm.samples, 17) < 0.01, "bin 17 (the exhaust's own pitch): {}", bin(&first.pcm.samples, 17));
}

#[test]
fn a_model_whose_loops_are_at_twice_the_redline_has_a_cycle_of_one_turn() {
    let parsed = rev::parse(&model(&usual())).unwrap();
    // The top loop is 90 Hz: 10800 rpm at 120 per cycle per second, which is under 1.5 times an 11000 redline.
    assert_eq!(rpm_per_cycle_hz(&parsed, 11_000.0), 120.0);
    assert_eq!(rpm_per_cycle_hz(&parsed, 7_200.0), 120.0, "1.5 times the redline is not above it");
    // For a 5400 rpm redline it is twice, so the loops are read at 60: their speeds are halved.
    assert_eq!(rpm_per_cycle_hz(&parsed, 5_400.0), 60.0);
    let layers =
        build(&parsed, &Tones, &parsed, &Tones, &Plan { loops: 3, idle_rpm: 1200.0, redline_rpm: 5_400.0 }).unwrap();
    let on = rpms(&layers, false);
    assert!((on[1] - 1800.0).abs() < 0.1 && (on[2] - 3600.0).abs() < 0.1 && (on[3] - 5400.0).abs() < 0.1, "{on:?}");
    assert!((on[0] - 1200.0).abs() < 30.0, "the idle is looked for at 60 per cycle per second too: {on:?}");
    assert!(layers.notes.iter().any(|note| note.contains("one turn")), "{:?}", layers.notes);
}

/// Like [`Tones`], but the idle (stream 9) is noise without a period.
struct NoisyIdle;

impl Recordings for NoisyIdle {
    fn decode(&self, number: usize) -> Result<Pcm> {
        if number != 9 {
            return Tones.decode(number);
        }
        let mut seed = 7_u32;
        let samples = (0..72_000)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 8) as f32 / 8_388_608.0 - 1.0) * 0.2
            })
            .collect();
        Ok(Pcm { sample_rate: 48_000, channels: 1, samples, loop_range: None })
    }
}

#[test]
fn an_idle_without_a_period_plays_at_the_physics_idle_or_is_left_out() {
    let parsed = rev::parse(&model(&usual())).unwrap();
    let low_idle = Plan { idle_rpm: 1500.0, ..plan(3) };
    let with = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, &low_idle).unwrap();
    assert_eq!(rpms(&with, false).len(), 4);
    assert_eq!(rpms(&with, false)[0], 1500.0);
    assert_eq!(rpms(&with, true)[0], 1500.0, "and the same idle plays off load");
    assert!(with.notes.iter().any(|note| note.contains("physics idle")));
    // A physics idle between two loops still gets its layer, in order, in each set.
    let high_idle = Plan { idle_rpm: 5000.0, ..plan(3) };
    let between = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, &high_idle).unwrap();
    for off in [false, true] {
        let set = rpms(&between, off);
        assert_eq!(set.len(), 4);
        assert!(set.windows(2).all(|pair| pair[0] < pair[1]), "{set:?}");
    }
    // An idle at the speed of a loop under load is left out, with its copy off load.
    let same = Plan { idle_rpm: 3600.0, ..plan(3) };
    let without = build(&parsed, &NoisyIdle, &parsed, &NoisyIdle, &same).unwrap();
    assert_eq!((rpms(&without, false).len(), rpms(&without, true).len()), (3, 3));
    assert!(without.notes.iter().any(|note| note.contains("no idle layer")));
}

#[test]
fn the_low_range_is_taken_densely_and_the_rest_evenly() {
    let speeds = [3000.0, 3500.0, 4000.0, 4500.0, 5000.0, 5500.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0];
    // 40 percent of a 12000 rpm redline is 4800: four loops are low, and they are all taken (4 of 11 is under half).
    let picked = choose(&speeds, 4800.0, 11);
    assert_eq!(&picked[..4], &[0, 1, 2, 3]);
    assert_eq!(picked.len(), 11);
    assert_eq!(picked.last(), Some(&11));
    assert!(picked.windows(2).all(|pair| pair[0] < pair[1]));
    // Many low loops: at most half of the layers.
    let many = [1000.0, 1100.0, 1200.0, 1300.0, 1400.0, 1500.0, 1600.0, 1700.0, 5000.0, 6000.0];
    let picked = choose(&many, 2000.0, 4);
    assert_eq!(picked.len(), 4);
    assert_eq!(picked.iter().filter(|&&index| index < 8).count(), 2);
    // Nothing low, or everything low: an even spread.
    assert_eq!(choose(&[5000.0, 6000.0, 7000.0], 1000.0, 3), [0, 1, 2]);
    assert_eq!(choose(&[500.0, 600.0, 700.0], 1000.0, 2), [0, 2]);
    assert!(choose(&[], 1000.0, 3).is_empty());
}

#[test]
fn silent_input_and_bad_counts_are_refused() {
    let parsed = rev::parse(&model(&usual())).unwrap();
    assert!(build(&parsed, &Tones, &parsed, &Tones, &plan(0)).is_err());
    assert!(build(&parsed, &Tones, &parsed, &Tones, &plan(LOOP_LAYERS + 1)).is_err());
    let mut silent = EngineLayers {
        layers: vec![EngineLayer {
            rpm: 3000.0,
            pcm: Pcm { sample_rate: 48_000, channels: 1, samples: vec![0.0; 100], loop_range: None },
            off_load: false,
        }],
        worst_detune: 0.0,
        notes: Vec::new(),
    };
    assert!(equalise(&mut silent, 0.9).is_err());
}
