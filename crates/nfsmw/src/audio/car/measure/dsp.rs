//! The numbers the offline sound measurement reads off a rendered car: levels, spectrum, pitch, smoothness and gaps.
//! All functions take mono samples (`mono`) and the sample rate.

/// The floor of every level, in dB: silence reads as this, not as minus infinity.
pub const FLOOR_DB: f32 = -120.0;

/// The mean of the two channels of an interleaved stereo render.
pub fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo.as_chunks::<2>().0.iter().map(|frame| 0.5 * (frame[0] + frame[1])).collect()
}

fn to_db(amplitude: f64) -> f32 {
    (20.0 * amplitude.max(1e-6).log10()) as f32
}

/// The level of `samples` in dB re full scale (root mean square).
pub fn rms_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return FLOOR_DB;
    }
    to_db((samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt())
}

/// The highest absolute sample in dB re full scale.
pub fn peak_db(samples: &[f32]) -> f32 {
    to_db(samples.iter().fold(0.0f64, |peak, s| peak.max(f64::from(s.abs()))))
}

/// How many samples are beyond full scale.
pub fn clipped(samples: &[f32]) -> usize {
    samples.iter().filter(|s| s.abs() > 1.0).count()
}

/// The level of each consecutive window of `seconds`.
pub fn window_db(samples: &[f32], rate: u32, seconds: f32) -> Vec<f32> {
    let size = ((rate as f32 * seconds) as usize).max(1);
    samples.chunks(size).filter(|chunk| chunk.len() == size).map(rms_db).collect()
}

/// A gated loudness, LUFS-like but without the K weighting: the mean power of the windows of 400 ms that are within 10 dB
/// of the loudest, in dB. It ignores pauses.
pub fn loudness_db(samples: &[f32], rate: u32) -> f32 {
    let windows = window_db(samples, rate, 0.4);
    let Some(loudest) = windows.iter().copied().reduce(f32::max) else { return FLOOR_DB };
    let kept: Vec<f64> =
        windows.iter().filter(|db| **db > loudest - 10.0).map(|db| 10f64.powf(f64::from(*db) / 10.0)).collect();
    (10.0 * (kept.iter().sum::<f64>() / kept.len() as f64).log10()) as f32
}

/// The spectral centroid in Hz: the power-weighted mean frequency over Hann windows of 4096 samples.
pub fn centroid_hz(samples: &[f32], rate: u32) -> f32 {
    const N: usize = 4096;
    if samples.len() < N {
        return 0.0;
    }
    let mut power = vec![0.0f64; N / 2];
    let mut start = 0;
    while start + N <= samples.len() {
        for (total, value) in power.iter_mut().zip(fft_power(&samples[start..start + N])) {
            *total += value;
        }
        start += N / 2;
    }
    let sum: f64 = power.iter().sum();
    if sum <= 0.0 {
        return 0.0;
    }
    let weighted: f64 = power.iter().enumerate().map(|(bin, value)| bin as f64 * value).sum();
    (weighted / sum * f64::from(rate) / N as f64) as f32
}

/// The frequency of the strongest part of the spectrum between `low` and `high` Hz, over Hann windows of 4096 samples.
pub fn peak_hz(samples: &[f32], rate: u32, low: f32, high: f32) -> f32 {
    const N: usize = 4096;
    if samples.len() < N {
        return 0.0;
    }
    let mut power = vec![0.0f64; N / 2];
    let mut start = 0;
    while start + N <= samples.len() {
        for (total, value) in power.iter_mut().zip(fft_power(&samples[start..start + N])) {
            *total += value;
        }
        start += N / 2;
    }
    let bin = |hz: f32| (hz * N as f32 / rate as f32) as usize;
    let (from, to) = (bin(low).max(1), bin(high).min(N / 2 - 1));
    let best = (from..=to).max_by(|a, b| power[*a].total_cmp(&power[*b])).unwrap_or(0);
    best as f32 * rate as f32 / N as f32
}

/// The power spectrum (the first half) of a Hann-windowed block of a power of two samples.
fn fft_power(block: &[f32]) -> Vec<f64> {
    let n = block.len();
    let mut re: Vec<f64> = block
        .iter()
        .enumerate()
        .map(|(i, s)| f64::from(*s) * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()))
        .collect();
    let mut im = vec![0.0f64; n];
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
        }
    }
    let mut size = 2;
    while size <= n {
        let angle = -std::f64::consts::TAU / size as f64;
        for start in (0..n).step_by(size) {
            for k in 0..size / 2 {
                let (sin, cos) = (angle * k as f64).sin_cos();
                let (a, b) = (start + k, start + k + size / 2);
                let (tr, ti) = (re[b] * cos - im[b] * sin, re[b] * sin + im[b] * cos);
                (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                (re[a], im[a]) = (re[a] + tr, im[a] + ti);
            }
        }
        size *= 2;
    }
    (0..n / 2).map(|i| re[i] * re[i] + im[i] * im[i]).collect()
}

/// The fundamental in Hz between `low` and `high`: the first local peak of the normalised autocorrelation that is nearly
/// as strong as the best one; 0 when nothing repeats (a noise).
pub fn fundamental_hz(samples: &[f32], rate: u32, low: f32, high: f32) -> f32 {
    let window = 16_384.min(samples.len() / 2);
    if window < 1024 {
        return 0.0;
    }
    let stretch = &samples[samples.len() - window..];
    let energy: f32 = stretch.iter().map(|s| s * s).sum();
    if energy < 1e-9 {
        return 0.0;
    }
    let (shortest, longest) = ((rate as f32 / high) as usize, ((rate as f32 / low) as usize).min(window / 2));
    if shortest + 2 >= longest {
        return 0.0;
    }
    let correlation: Vec<f32> = (shortest..longest)
        .map(|lag| stretch[..window - lag].iter().zip(&stretch[lag..]).map(|(a, b)| a * b).sum::<f32>() / energy)
        .collect();
    let best = correlation.iter().copied().fold(f32::MIN, f32::max);
    if best < 0.25 {
        return 0.0;
    }
    let peak = (1..correlation.len() - 1).find(|&at| {
        correlation[at] >= 0.85 * best
            && correlation[at] >= correlation[at - 1]
            && correlation[at] > correlation[at + 1]
    });
    peak.map_or(0.0, |at| rate as f32 / (shortest + at) as f32)
}

/// The largest jump in level between two consecutive windows of `seconds`, in dB, over the windows that are above
/// `floor_db`: a crossfade glitch or a layer that switches on shows as a big step.
pub fn max_level_step_db(samples: &[f32], rate: u32, seconds: f32, floor_db: f32) -> f32 {
    window_db(samples, rate, seconds)
        .windows(2)
        .filter(|pair| pair[0] > floor_db && pair[1] > floor_db)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

/// The sample-level discontinuity: the largest step between two samples, relative to the root mean square of the 20 ms
/// around it. A click is far above what the signal's own slope allows.
pub fn click_ratio(samples: &[f32], rate: u32) -> f32 {
    let window = (rate as usize / 50).max(8);
    let mut worst = 0.0f32;
    for (block, chunk) in samples.chunks(window).enumerate() {
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        if rms < 1e-4 {
            continue;
        }
        let from = (block * window).saturating_sub(1);
        let to = (block * window + chunk.len()).min(samples.len());
        let step = samples[from..to].windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0f32, f32::max);
        worst = worst.max(step / rms);
    }
    worst
}

/// How many windows of `seconds` fall below `floor_db` while their neighbours on both sides are above it: a hole in a
/// sound that should be continuous.
pub fn gaps(samples: &[f32], rate: u32, seconds: f32, floor_db: f32) -> usize {
    let levels = window_db(samples, rate, seconds);
    levels.windows(3).filter(|w| w[1] < floor_db && w[0] > floor_db && w[2] > floor_db).count()
}

/// How many times the level jumps more than `rise_db` above the median of the 10 windows before it (windows of
/// `seconds`): the shifts, pops and backfires of a drive. Each burst is counted once.
pub fn bursts(samples: &[f32], rate: u32, seconds: f32, rise_db: f32) -> usize {
    let levels = window_db(samples, rate, seconds);
    let mut count = 0;
    let mut cooldown = 0;
    for at in 10..levels.len() {
        if cooldown > 0 {
            cooldown -= 1;
            continue;
        }
        let mut before: Vec<f32> = levels[at - 10..at].to_vec();
        before.sort_by(f32::total_cmp);
        if levels[at] > before[5] + rise_db {
            count += 1;
            cooldown = 5;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn sine(hz: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize)
            .map(|i| amplitude * (std::f32::consts::TAU * hz * i as f32 / RATE as f32).sin())
            .collect()
    }

    #[test]
    fn a_sine_has_its_level_peak_and_pitch() {
        let tone = sine(120.0, 1.0, 0.5);
        assert!((rms_db(&tone) - (-9.03)).abs() < 0.1, "{}", rms_db(&tone));
        assert!((peak_db(&tone) - (-6.02)).abs() < 0.1);
        assert!((fundamental_hz(&tone, RATE, 25.0, 500.0) - 120.0).abs() < 2.0);
        assert!((loudness_db(&tone, RATE) - rms_db(&tone)).abs() < 0.1);
        assert!(clipped(&sine(100.0, 0.1, 1.5)) > 0);
        assert_eq!(clipped(&tone), 0);
    }

    #[test]
    fn the_spectral_peak_of_a_chord_is_its_strongest_note() {
        let mut chord = sine(150.0, 1.0, 0.5);
        for (sample, other) in chord.iter_mut().zip(sine(400.0, 1.0, 0.2)) {
            *sample += other;
        }
        assert!((peak_hz(&chord, RATE, 40.0, 1000.0) - 150.0).abs() < 15.0);
        assert!((peak_hz(&chord, RATE, 300.0, 1000.0) - 400.0).abs() < 15.0);
    }

    #[test]
    fn the_centroid_of_a_sine_is_its_frequency() {
        let low = centroid_hz(&sine(300.0, 1.0, 0.5), RATE);
        let high = centroid_hz(&sine(3000.0, 1.0, 0.5), RATE);
        assert!((low - 300.0).abs() < 30.0 && (high - 3000.0).abs() < 60.0, "{low} {high}");
    }

    #[test]
    fn silence_is_the_floor_and_noise_has_no_pitch() {
        assert_eq!(rms_db(&[0.0; 100]), -120.0);
        let mut state = 0x1234_5678u32;
        let noise: Vec<f32> = (0..48_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect();
        assert_eq!(fundamental_hz(&noise, RATE, 25.0, 500.0), 0.0);
    }

    #[test]
    fn a_step_in_level_and_a_click_and_a_hole_are_found() {
        let mut signal = sine(100.0, 1.0, 0.1);
        signal.extend(sine(100.0, 1.0, 0.4));
        assert!((max_level_step_db(&signal, RATE, 0.05, -60.0) - 12.0).abs() < 1.0);
        let smooth = click_ratio(&sine(100.0, 1.0, 0.3), RATE);
        let mut clicked = sine(100.0, 1.0, 0.3);
        clicked[24_000] += 0.8;
        assert!(click_ratio(&clicked, RATE) > 4.0 * smooth, "{} {smooth}", click_ratio(&clicked, RATE));
        let mut holed = sine(100.0, 1.0, 0.3);
        holed[14_400..16_800].fill(0.0);
        assert_eq!(gaps(&holed, RATE, 0.05, -60.0), 1);
        assert_eq!(gaps(&sine(100.0, 1.0, 0.3), RATE, 0.05, -60.0), 0);
    }

    #[test]
    fn bursts_are_counted_once_each() {
        let mut signal = sine(100.0, 3.0, 0.05);
        for at in [30_000usize, 90_000] {
            for (i, s) in sine(400.0, 0.1, 0.5).iter().enumerate() {
                signal[at + i] += s;
            }
        }
        assert_eq!(bursts(&signal, RATE, 0.02, 9.0), 2);
    }
}
