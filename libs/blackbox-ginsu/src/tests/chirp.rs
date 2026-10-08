//! A synthetic `.gin`: a sine whose frequency rises linearly from `hz0` to `hz1` over `seconds` and keeps
//! rising for 5 % longer (a real recording runs a little past its top frequency). Cycle boundaries sit at
//! whole turns of phase and the frequency table follows the known frequency law, exactly the shape of a
//! recorded rev-up. Frequencies in the tables are in the file's units (Hz x 120).

use std::sync::Arc;

use crate::{FREQUENCY_PER_HZ, GinsuData, GinsuTables};

pub const AMPLITUDE: f32 = 0.5;

pub struct Chirp {
    pub data: Arc<GinsuData>,
    pub sample_rate: u32,
    pub hz0: f64,
    pub hz1: f64,
}

impl Chirp {
    /// Frequency (file units) at fraction `x` (0 to 1) of the range.
    pub fn freq(&self, x: f32) -> f32 {
        let hz = self.hz0 + (self.hz1 - self.hz0) * f64::from(x);
        hz as f32 * FREQUENCY_PER_HZ
    }
}

pub fn chirp(sample_rate: u32, seconds: f64, hz0: f64, hz1: f64) -> Chirp {
    let k = (hz1 - hz0) / seconds;
    let rate = f64::from(sample_rate);
    let turns = |t: f64| hz0 * t + 0.5 * k * t * t;
    let cycles = turns(seconds * 1.05).floor() as usize;
    let time_of_turn = |c: f64| (-hz0 + (hz0 * hz0 + 2.0 * k * c).sqrt()) / k;
    let cycle_pos: Vec<u32> = (0..=cycles).map(|c| (time_of_turn(c as f64) * rate).round() as u32).collect();
    let sample_count = *cycle_pos.last().unwrap() + 1;
    let freq_pos: Vec<u32> = (0..=50)
        .map(|i| {
            let hz = hz0 + (hz1 - hz0) * f64::from(i) / 50.0;
            (((hz - hz0) / k * rate).round() as u32).min(sample_count - 1)
        })
        .collect();
    let pcm = (0..sample_count)
        .map(|n| {
            let t = f64::from(n) / rate;
            (AMPLITUDE as f64 * (std::f64::consts::TAU * turns(t)).sin()) as f32
        })
        .collect();
    let tables = GinsuTables::new(
        (hz0 * f64::from(FREQUENCY_PER_HZ)) as f32,
        (hz1 * f64::from(FREQUENCY_PER_HZ)) as f32,
        sample_rate,
        sample_count,
        freq_pos,
        cycle_pos,
    )
    .unwrap();
    Chirp { data: Arc::new(GinsuData::new(tables, pcm).unwrap()), sample_rate, hz0, hz1 }
}

/// A loop like the shipped ones: 32 kHz, 10 to 60 Hz (1200 to 7200 in file units) over 5 seconds.
pub fn standard() -> Chirp {
    chirp(32_000, 5.0, 10.0, 60.0)
}

/// Mean frequency of `signal` in Hz from its rising zero crossings (linear interpolation).
pub fn measure_hz(signal: &[f32], sample_rate: u32) -> f32 {
    let mut first = None;
    let mut last = 0.0f32;
    let mut count = 0;
    for (i, w) in signal.windows(2).enumerate() {
        if w[0] < 0.0 && w[1] >= 0.0 {
            let at = i as f32 + -w[0] / (w[1] - w[0]);
            first.get_or_insert(at);
            last = at;
            count += 1;
        }
    }
    let Some(first) = first else { return 0.0 };
    if count < 2 {
        return 0.0;
    }
    (count - 1) as f32 * sample_rate as f32 / (last - first)
}

/// The largest difference between neighbouring samples.
pub fn max_step(signal: &[f32]) -> f32 {
    signal.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max)
}
