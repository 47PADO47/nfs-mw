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

/// A four-point cubic read of a periodic loop at a fractional `position` (in frames; it wraps round the loop).
fn cubic_at(samples: &[f32], position: f64) -> f32 {
    let n = samples.len() as isize;
    let at = |index: isize| samples[index.rem_euclid(n) as usize];
    let base = position.floor();
    let t = (position - base) as f32;
    let i = base as isize;
    let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
    0.5 * ((2.0 * p1)
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (3.0 * (p1 - p2) + p3 - p0) * t * t * t)
}

/// Resample a loop to `frames` frames, reading it as periodic (the position wraps), with a four-point cubic. The loop's
/// speed changes by `samples.len() / frames` and its seam stays seamless.
pub fn resample_periodic(samples: &[f32], frames: usize) -> Vec<f32> {
    let n = samples.len();
    if n == 0 || frames == 0 {
        return Vec::new();
    }
    (0..frames).map(|frame| cubic_at(samples, frame as f64 * n as f64 / frames as f64)).collect()
}

/// Frames over which [`retime_periodic`] closes the seam of its result (2 ms at 48 kHz).
const SEAM_FRAMES: usize = 96;

/// Read a periodic loop of `cycles_in` whole engine cycles so that `frames` frames hold `cycles_out` cycles of it: the
/// result's pitch is exactly `cycles_out` over its length, whatever the loop's own pitch was (it is retimed by
/// `cycles_out / cycles_in` of the loop's own length). The loop is read round and round, so after `frames` frames the read
/// is at cycle `cycles_out mod cycles_in`, not at the start; the last [`SEAM_FRAMES`] frames fade into the same signal
/// read from the start, so the result loops without a click. Empty for an empty loop or a zero count.
pub fn retime_periodic(samples: &[f32], cycles_in: u32, cycles_out: u32, frames: usize) -> Vec<f32> {
    let n = samples.len();
    if n == 0 || frames == 0 || cycles_in == 0 || cycles_out == 0 {
        return Vec::new();
    }
    // Loop frames read per output frame, and where the plain read ends.
    let step = n as f64 * f64::from(cycles_out) / (f64::from(cycles_in) * frames as f64);
    let total = frames as f64 * step;
    let seam = SEAM_FRAMES.min(frames / 4);
    let fade_from = frames - seam;
    (0..frames)
        .map(|frame| {
            let position = frame as f64 * step;
            let here = cubic_at(samples, position);
            if frame < fade_from {
                return here;
            }
            let weight = (frame - fade_from + 1) as f32 / seam as f32;
            here + (cubic_at(samples, position - total) - here) * weight
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

    /// The size of one Fourier bin of `samples` (a cosine of amplitude 1 on a bin is 0.5).
    fn bin(samples: &[f32], k: usize) -> f32 {
        let n = samples.len() as f32;
        let (re, im) = samples.iter().enumerate().fold((0.0_f32, 0.0_f32), |(re, im), (i, x)| {
            let angle = std::f32::consts::TAU * (k * i) as f32 / n;
            (re + x * angle.cos(), im - x * angle.sin())
        });
        re.hypot(im) / n
    }

    #[test]
    fn a_retimed_loop_has_exactly_the_wanted_number_of_cycles_in_its_length() {
        // 11 cycles in 4800 frames read as 10 cycles in 5000 frames: all of the energy is in bin 10, none in bin 11.
        let source: Vec<f32> = (0..4800).map(|i| (std::f32::consts::TAU * 11.0 * i as f32 / 4800.0).cos()).collect();
        let out = retime_periodic(&source, 11, 10, 5000);
        assert_eq!(out.len(), 5000);
        assert!((bin(&out, 10) - 0.5).abs() < 0.01, "bin 10: {}", bin(&out, 10));
        assert!(bin(&out, 11) < 0.01 && bin(&out, 9) < 0.01, "{} {}", bin(&out, 11), bin(&out, 9));
        // Resampling to the same length keeps the loop's own 11 cycles: the pitch moves with the length, which retiming undoes.
        let plain = resample_periodic(&source, 5000);
        assert!(bin(&plain, 11) > 0.45 && bin(&plain, 10) < 0.01);
    }

    #[test]
    fn a_retimed_loop_with_other_cycles_in_it_still_closes_its_seam() {
        // A loop whose cycles differ (the level swells once round the loop): reading it for another number of cycles ends
        // in another cycle than it starts in. The seam is faded, so the end meets the start.
        let n = 4800;
        let source: Vec<f32> = (0..n)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / n as f32;
                (11.0 * phase).cos() * (1.0 + 0.5 * phase.sin())
            })
            .collect();
        let out = retime_periodic(&source, 11, 7, 3000);
        let step = out.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0_f32, f32::max);
        let seam = (out[0] - out[out.len() - 1]).abs();
        assert!(seam <= step * 1.5, "the seam {seam} is no bigger than the largest step {step}");
        let read_step = n as f64 * 7.0 / (11.0 * 3000.0);
        let unfaded = (cubic_at(&source, 2999.0 * read_step) - source[0]).abs();
        assert!(unfaded > seam * 2.0, "without the fade the jump would be {unfaded}");
        assert!(retime_periodic(&[], 11, 7, 10).is_empty());
        assert!(retime_periodic(&source, 0, 7, 10).is_empty());
        assert!(retime_periodic(&source, 11, 7, 0).is_empty());
    }
}
