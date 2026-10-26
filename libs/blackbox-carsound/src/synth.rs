//! A built-in engine sound: seamless loops synthesised from nothing but the engine's firing order, for a car that has no
//! recording of its own engine. No sample is read and none belongs to any car.
//!
//! A four-stroke engine repeats its sound every two turns of the crankshaft (one *cycle*) and fires once per cylinder in
//! it. The loop spans a whole number of cycles, and every component of it is a whole multiple of the cycle rate, so the
//! loop joins its own start without a click. The firing harmonics (multiples of cylinders per cycle) carry the tone,
//! the other multiples of the cycle add the uneven "lope" of a real engine, and a little low-passed noise the roughness.
//! The sound brightens with the engine speed.

use std::f32::consts::TAU;

/// The shortest loop, in seconds: shorter ones would repeat fast enough to be heard as a tone of their own.
const MIN_SECONDS: f32 = 0.5;
/// The level (root mean square) every loop is brought to, so layers of one engine mix without a jump.
const TARGET_RMS: f32 = 0.2;
/// No sample goes above this.
const PEAK_LIMIT: f32 = 0.95;
/// Harmonics stop below this fraction of the sample rate.
const BANDWIDTH: f32 = 0.45;
/// A harmonic weaker than this (the loudest firing harmonic is 1) adds nothing a listener hears, and is left out.
const AUDIBLE: f32 = 0.003;

/// One loop of an engine of `cylinders` running at `rpm`, as samples between -1 and 1 at `sample_rate`.
/// Empty for an rpm, cylinder count or rate that is not a positive number.
pub fn engine_loop(rpm: f32, cylinders: u32, sample_rate: u32) -> Vec<f32> {
    if !(rpm.is_finite() && rpm > 0.0) || cylinders == 0 || sample_rate == 0 {
        return Vec::new();
    }
    let cycle_hz = rpm / 120.0;
    let cycles = (MIN_SECONDS * cycle_hz).ceil().max(1.0);
    let length = (cycles * sample_rate as f32 / cycle_hz).round() as usize;
    let bright = ((rpm - 600.0) / 6000.0).clamp(0.0, 1.0);
    let corner = 1200.0 + 3000.0 * bright;
    let mut out = vec![0.0f32; length];
    for harmonic in 1u32.. {
        let hz = harmonic as f32 * cycle_hz;
        if hz > BANDWIDTH * sample_rate as f32 {
            break;
        }
        let firing = harmonic % cylinders == 0;
        let order = (harmonic / cylinders).max(1) as f32;
        let level = match firing {
            true => order.powf(-1.15),
            false => 0.12 / (harmonic as f32).sqrt(),
        };
        let amplitude = level / (1.0 + (hz / corner).powi(2));
        if amplitude < AUDIBLE {
            continue;
        }
        // Firing harmonics line up, which gives the sharp pulse of a firing; the others start anywhere.
        let start = if firing { 0.0 } else { TAU * unit(harmonic) };
        let step = f64::from(TAU * harmonic as f32 * cycles / length as f32);
        // A sine by rotating a phasor: one multiplication per sample instead of a `sin`.
        let (sin_step, cos_step) = step.sin_cos();
        let (mut sin, mut cos) = f64::from(start).sin_cos();
        for sample in out.iter_mut() {
            *sample += amplitude * sin as f32;
            (sin, cos) = (sin * cos_step + cos * sin_step, cos * cos_step - sin * sin_step);
        }
    }
    add_noise(&mut out, 0.04 + 0.08 * bright);
    level_off(&mut out);
    out
}

/// A repeatable number between 0 and 1 for `seed`.
fn unit(seed: u32) -> f32 {
    let mixed = seed.wrapping_mul(2_654_435_761).rotate_left(13).wrapping_mul(2_246_822_519);
    (mixed >> 8) as f32 / (1u32 << 24) as f32
}

/// Low-passed noise at `gain`, whose end runs on into its start.
fn add_noise(samples: &mut [f32], gain: f32) {
    let length = samples.len();
    let mut state = 0x1357_9bdfu32;
    let white: Vec<f32> = (0..length)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / (1u32 << 23) as f32 - 1.0
        })
        .collect();
    let mut low = 0.0f32;
    for pass in 0..2 {
        for (at, noise) in white.iter().enumerate() {
            low += 0.25 * (noise - low);
            if pass == 1 {
                samples[at] += gain * low;
            }
        }
    }
}

/// Scale `samples` to [`TARGET_RMS`], or lower if that would pass [`PEAK_LIMIT`].
fn level_off(samples: &mut [f32]) {
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
    if rms <= 1e-6 || peak <= 1e-6 {
        return;
    }
    let gain = (TARGET_RMS / rms).min(PEAK_LIMIT / peak);
    for sample in samples {
        *sample *= gain;
    }
}

/// 16-bit samples of `samples`.
pub fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 32_000;

    /// The strength of `hz` in `samples` (Goertzel).
    fn strength(samples: &[f32], hz: f32) -> f32 {
        let w = TAU * hz / RATE as f32;
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &x in samples {
            let s = x + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2).max(0.0).sqrt() / samples.len() as f32
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn the_loop_holds_whole_cycles_and_is_at_least_half_a_second() {
        for rpm in [900.0f32, 2500.0, 6500.0] {
            let samples = engine_loop(rpm, 6, RATE);
            let cycle = RATE as f32 * 120.0 / rpm;
            let cycles = samples.len() as f32 / cycle;
            assert!((cycles - cycles.round()).abs() < 0.01, "{rpm} rpm: {cycles} cycles");
            assert!(samples.len() as f32 >= 0.5 * RATE as f32 - 1.0, "{rpm} rpm: {} samples", samples.len());
        }
    }

    #[test]
    fn the_strongest_tone_is_the_firing_rate() {
        for (rpm, cylinders) in [(900.0f32, 6u32), (3000.0, 6), (6500.0, 6), (2000.0, 4), (4000.0, 8)] {
            let samples = engine_loop(rpm, cylinders, RATE);
            let fire = rpm / 120.0 * cylinders as f32;
            let at_fire = strength(&samples, fire);
            for other in [0.5, 0.75, 1.5, 1.25] {
                assert!(
                    at_fire > 2.0 * strength(&samples, fire * other),
                    "{rpm} rpm {cylinders} cyl: {at_fire} at {fire} Hz against {other} times it"
                );
            }
        }
    }

    #[test]
    fn the_loop_joins_its_own_start_without_a_click() {
        for rpm in [900.0f32, 3000.0, 6500.0] {
            let samples = engine_loop(rpm, 6, RATE);
            let widest = samples.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0f32, f32::max);
            let join = (samples[0] - samples[samples.len() - 1]).abs();
            assert!(join <= widest, "{rpm} rpm: the join jumps {join}, the widest step inside is {widest}");
        }
    }

    #[test]
    fn every_loop_has_the_same_level_and_stays_in_range() {
        for rpm in [700.0f32, 1800.0, 4200.0, 7000.0] {
            let samples = engine_loop(rpm, 6, RATE);
            assert!(samples.iter().all(|s| s.abs() <= 0.951), "{rpm} rpm clips");
            assert!((rms(&samples) - TARGET_RMS).abs() < 0.02, "{rpm} rpm: {}", rms(&samples));
        }
    }

    #[test]
    fn the_sound_brightens_with_the_engine_speed() {
        // How much of the level is in fast changes: the first difference against the signal.
        let roughness = |rpm: f32| {
            let samples = engine_loop(rpm, 6, RATE);
            let steps: Vec<f32> = samples.windows(2).map(|pair| pair[1] - pair[0]).collect();
            rms(&steps) / rms(&samples)
        };
        assert!(roughness(6500.0) > 1.5 * roughness(900.0), "{} against {}", roughness(6500.0), roughness(900.0));
    }

    #[test]
    fn the_same_engine_always_sounds_the_same() {
        assert_eq!(engine_loop(3000.0, 6, RATE), engine_loop(3000.0, 6, RATE));
    }

    #[test]
    fn nonsense_makes_no_sound() {
        assert!(engine_loop(0.0, 6, RATE).is_empty());
        assert!(engine_loop(f32::NAN, 6, RATE).is_empty());
        assert!(engine_loop(3000.0, 0, RATE).is_empty());
        assert!(engine_loop(3000.0, 6, 0).is_empty());
    }

    #[test]
    fn sixteen_bit_samples_clamp() {
        assert_eq!(to_pcm16(&[0.0, 1.0, -1.0, 2.0, -0.5]), [0, 32767, -32767, 32767, -16384]);
    }
}
