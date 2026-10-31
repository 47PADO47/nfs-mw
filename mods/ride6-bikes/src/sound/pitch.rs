//! Two small signal tools for the engine recordings: the pitch of a steady engine clip, and the resampling of a loop to a
//! given length.

/// The frequency (Hz) of the engine cycle in a steady clip, by the autocorrelation of its low band; `None` when the clip
/// is too short or has no clear period between `low_hz` and `high_hz`.
///
/// The clip is averaged down to about 4 kHz first (an engine's cycle is far below that and its harmonics up to 2 kHz carry
/// the period), the normalised autocorrelation is searched from the lag of `high_hz` to that of `low_hz`, the first peak at
/// 80 percent of the best one is taken (a clip of many cycles also correlates at their multiples), and its position is
/// refined with a parabola through its neighbours.
pub fn cycle_frequency(samples: &[f32], sample_rate: u32, low_hz: f32, high_hz: f32) -> Option<f32> {
    if samples.is_empty() || sample_rate == 0 || low_hz <= 0.0 || high_hz <= low_hz {
        return None;
    }
    let factor = (sample_rate / 4000).max(1) as usize;
    let rate = sample_rate as f32 / factor as f32;
    let reduced: Vec<f32> =
        samples.chunks_exact(factor).map(|chunk| chunk.iter().sum::<f32>() / factor as f32).collect();
    let mean = reduced.iter().sum::<f32>() / reduced.len().max(1) as f32;
    let centred: Vec<f32> = reduced.iter().map(|sample| sample - mean).collect();
    let shortest = (rate / high_hz).floor().max(2.0) as usize;
    let longest = (rate / low_hz).ceil() as usize;
    if centred.len() < longest * 3 {
        return None;
    }
    let zero: f32 = centred.iter().map(|s| s * s).sum();
    if zero <= 1e-12 {
        return None;
    }
    let correlation = |lag: usize| -> f32 {
        let pairs = centred.len() - lag;
        let sum: f32 = centred[..pairs].iter().zip(&centred[lag..]).map(|(a, b)| a * b).sum();
        sum / zero * centred.len() as f32 / pairs as f32
    };
    let values: Vec<f32> = (shortest..=longest).map(correlation).collect();
    let best = values.iter().copied().fold(f32::MIN, f32::max);
    if best < 0.2 {
        return None;
    }
    let peak = (1..values.len().saturating_sub(1))
        .find(|&n| values[n] > values[n - 1] && values[n] >= values[n + 1] && values[n] >= 0.8 * best)?;
    let (before, here, after) = (values[peak - 1], values[peak], values[peak + 1]);
    let curve = before - 2.0 * here + after;
    let shift = if curve.abs() > 1e-9 { 0.5 * (before - after) / curve } else { 0.0 };
    let lag = (shortest + peak) as f32 + shift;
    Some(rate / lag)
}

/// Resample a loop to `frames` frames, reading it as periodic (the position wraps), with a four-point cubic. The loop's
/// speed changes by `samples.len() / frames` and its seam stays seamless.
pub fn resample_periodic(samples: &[f32], frames: usize) -> Vec<f32> {
    let n = samples.len();
    if n == 0 || frames == 0 {
        return Vec::new();
    }
    let at = |index: isize| samples[index.rem_euclid(n as isize) as usize];
    (0..frames)
        .map(|frame| {
            let position = frame as f64 * n as f64 / frames as f64;
            let base = position.floor();
            let t = (position - base) as f32;
            let i = base as isize;
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            0.5 * ((2.0 * p1)
                + (p2 - p0) * t
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
                + (3.0 * (p1 - p2) + p3 - p0) * t * t * t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, rate: u32, seconds: f32) -> Vec<f32> {
        let frames = (rate as f32 * seconds) as usize;
        (0..frames)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (std::f32::consts::TAU * hz * t).sin() + 0.5 * (std::f32::consts::TAU * 2.0 * hz * t).sin()
            })
            .collect()
    }

    #[test]
    fn the_cycle_frequency_of_a_steady_tone_is_found() {
        for hz in [17.4_f32, 26.5, 60.0, 94.7] {
            let found = cycle_frequency(&tone(hz, 48_000, 1.5), 48_000, 8.0, 140.0).unwrap();
            assert!((found - hz).abs() / hz < 0.01, "{hz} found as {found}");
        }
    }

    #[test]
    fn silence_noise_and_short_clips_have_no_pitch() {
        assert!(cycle_frequency(&[0.0; 48_000], 48_000, 8.0, 140.0).is_none());
        assert!(cycle_frequency(&tone(60.0, 48_000, 0.01), 48_000, 8.0, 140.0).is_none());
        assert!(cycle_frequency(&[], 48_000, 8.0, 140.0).is_none());
        assert!(cycle_frequency(&tone(60.0, 48_000, 1.0), 48_000, 60.0, 8.0).is_none());
        let mut seed = 1_u32;
        let noise: Vec<f32> = (0..48_000)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect();
        assert!(cycle_frequency(&noise, 48_000, 8.0, 140.0).is_none());
    }

    #[test]
    fn a_periodic_loop_keeps_its_cycles_when_resampled_and_stays_seamless() {
        // 10 cycles in 4800 frames, stretched to 5000: still 10 cycles, the last sample next to the first.
        let source: Vec<f32> = (0..4800).map(|i| (std::f32::consts::TAU * 10.0 * i as f32 / 4800.0).sin()).collect();
        let out = resample_periodic(&source, 5000);
        assert_eq!(out.len(), 5000);
        let expected: Vec<f32> = (0..5000).map(|i| (std::f32::consts::TAU * 10.0 * i as f32 / 5000.0).sin()).collect();
        let worst = out.iter().zip(&expected).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
        assert!(worst < 0.01, "worst error {worst}");
        assert!(resample_periodic(&[], 10).is_empty());
        assert!(resample_periodic(&source, 0).is_empty());
    }
}
