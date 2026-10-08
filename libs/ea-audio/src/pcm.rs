//! Decoded audio.

/// Decoded 16-bit PCM, interleaved when `channels > 1`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pcm {
    /// Samples per second per channel.
    pub sample_rate: u32,
    /// Number of interleaved channels.
    pub channels: u16,
    /// Interleaved samples: `frames() * channels` values.
    pub samples: Vec<i16>,
    /// Loop points as `(start, end)` in frames (`end` exclusive), when the stream declares them.
    /// The decoder does not loop; the player does.
    pub loop_range: Option<(u32, u32)>,
}

impl Pcm {
    /// Number of sample frames (one sample per channel).
    pub fn frames(&self) -> usize {
        match self.channels {
            0 => 0,
            c => self.samples.len() / c as usize,
        }
    }

    /// Length in seconds.
    pub fn duration_secs(&self) -> f64 {
        match self.sample_rate {
            0 => 0.0,
            r => self.frames() as f64 / r as f64,
        }
    }

    /// The samples scaled to `-1.0..1.0`.
    pub fn to_f32(&self) -> Vec<f32> {
        self.samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }
}
