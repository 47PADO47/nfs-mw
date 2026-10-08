//! The tables of a `Gnsu` file: where in the recording each frequency and each pitch cycle is.
//! Spec: `docs/specs/engine-sound-ginsu.md` §1 and `docs/formats/audio.md`.

use crate::round;
use crate::{Error, Result};

/// `"Gnsu"`.
pub const MAGIC: [u8; 4] = *b"Gnsu";
/// Bytes before the first table.
pub const HEADER_LEN: usize = 0x20;
/// Frequency units per Hz of the recording's fundamental: a cycle is `sample_rate * 120 / frequency`
/// samples long (the engine's "RPM" is `Hz x 120`).
pub const FREQUENCY_PER_HZ: f32 = 120.0;

/// Bytes of EA-XAS data per 32 samples, the payload that follows the tables in a file.
const XAS_BLOCK_BYTES: usize = 0x13;
const XAS_BLOCK_SAMPLES: usize = 32;

/// Frequency and cycle tables, with the recording's size and rate.
#[derive(Debug, Clone, PartialEq)]
pub struct GinsuTables {
    min_frequency: f32,
    max_frequency: f32,
    sample_rate: u32,
    sample_count: u32,
    /// `seg_count + 1` sample indices: where the pitch equals `min + (max - min) * i / seg_count`.
    freq_pos: Vec<u32>,
    /// `cycle_count + 1` sample indices of the cycle boundaries, strictly increasing.
    cycle_pos: Vec<u32>,
}

impl GinsuTables {
    /// Builds tables from their parts (the file parser and tests both come through here).
    ///
    /// `freq_pos` needs one entry at least; `cycle_pos` must be strictly increasing; the frequencies must be
    /// finite with `min <= max`.
    pub fn new(
        min_frequency: f32,
        max_frequency: f32,
        sample_rate: u32,
        sample_count: u32,
        freq_pos: Vec<u32>,
        cycle_pos: Vec<u32>,
    ) -> Result<Self> {
        if !min_frequency.is_finite() || !max_frequency.is_finite() || min_frequency > max_frequency {
            return Err(Error::InvalidTables("frequency range is not finite or is reversed"));
        }
        if freq_pos.is_empty() {
            return Err(Error::InvalidTables("the frequency table is empty"));
        }
        if cycle_pos.is_empty() {
            return Err(Error::InvalidTables("the cycle table is empty"));
        }
        if cycle_pos.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::InvalidTables("cycle positions are not strictly increasing"));
        }
        Ok(Self { min_frequency, max_frequency, sample_rate, sample_count, freq_pos, cycle_pos })
    }

    /// Parses the header and the two tables. Returns them with the offset of the sample payload.
    pub fn parse(bytes: &[u8]) -> Result<(Self, usize)> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated { needed: HEADER_LEN as u64, have: bytes.len() });
        }
        if bytes[..4] != MAGIC {
            return Err(Error::BadMagic);
        }
        if bytes[4] != b'2' {
            return Err(Error::UnsupportedVersion(bytes[4]));
        }
        let min_frequency = f32::from_le_bytes(word(bytes, 0x08));
        let max_frequency = f32::from_le_bytes(word(bytes, 0x0C));
        let seg_count = u32::from_le_bytes(word(bytes, 0x10));
        let cycle_count = u32::from_le_bytes(word(bytes, 0x14));
        let sample_count = u32::from_le_bytes(word(bytes, 0x18));
        let sample_rate = u32::from_le_bytes(word(bytes, 0x1C));
        let table_bytes = (u64::from(seg_count) + 1 + u64::from(cycle_count) + 1) * 4;
        let needed = HEADER_LEN as u64 + table_bytes;
        if (bytes.len() as u64) < needed {
            return Err(Error::Truncated { needed, have: bytes.len() });
        }
        let freq_end = HEADER_LEN + (seg_count as usize + 1) * 4;
        let freq_pos = read_u32s(&bytes[HEADER_LEN..freq_end]);
        let cycle_pos = read_u32s(&bytes[freq_end..needed as usize]);
        let tables = Self::new(min_frequency, max_frequency, sample_rate, sample_count, freq_pos, cycle_pos)?;
        Ok((tables, needed as usize))
    }

    pub fn min_frequency(&self) -> f32 {
        self.min_frequency
    }

    pub fn max_frequency(&self) -> f32 {
        self.max_frequency
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Segments of the frequency table (`freq_pos` has one more entry).
    pub fn seg_count(&self) -> usize {
        self.freq_pos.len() - 1
    }

    /// Pitch cycles of the recording (`cycle_pos` has one more entry).
    pub fn cycle_count(&self) -> usize {
        self.cycle_pos.len() - 1
    }

    pub fn freq_pos(&self) -> &[u32] {
        &self.freq_pos
    }

    pub fn cycle_pos(&self) -> &[u32] {
        &self.cycle_pos
    }

    /// Size in bytes of the EA-XAS payload that follows the tables in a file: 19 bytes per 32 samples.
    pub fn xas_payload_len(&self) -> usize {
        (self.sample_count as usize).div_ceil(XAS_BLOCK_SAMPLES) * XAS_BLOCK_BYTES
    }

    /// The sample at which the recording's pitch equals `frequency`; clamps to the ends of the range.
    pub fn frequency_to_sample(&self, frequency: f32) -> i32 {
        let seg = self.seg_count();
        if seg < 1 || frequency.is_nan() {
            return self.freq_at(0);
        }
        if frequency <= self.min_frequency {
            return self.freq_at(0);
        }
        if frequency >= self.max_frequency {
            return self.freq_at(seg);
        }
        let x = seg as f32 * (frequency - self.min_frequency) / (self.max_frequency - self.min_frequency);
        let i = (round::floor(x).max(0) as usize).min(seg - 1);
        let a = x - i as f32;
        let span = self.freq_pos[i + 1] as i64 - self.freq_pos[i] as i64;
        round::round(self.freq_pos[i] as f32 + a * span as f32)
    }

    /// The sample at fractional cycle index `cycle` (0 to `cycle_count`); clamps to the ends.
    pub fn cycle_to_sample(&self, cycle: f32) -> i32 {
        let n = self.cycle_count();
        if n < 1 || cycle.is_nan() || cycle <= 0.0 {
            return self.cycle_at(0);
        }
        if cycle >= n as f32 {
            return self.cycle_at(n);
        }
        let i = round::floor(cycle) as usize;
        let span = self.cycle_pos[i + 1] as i64 - self.cycle_pos[i] as i64;
        round::round(self.cycle_pos[i] as f32 + (cycle - i as f32) * span as f32)
    }

    /// The fractional cycle index of `sample`: the inverse of [`Self::cycle_to_sample`].
    pub fn sample_to_cycle(&self, sample: i32) -> f32 {
        let n = self.cycle_count();
        if n < 1 || i64::from(sample) <= i64::from(self.cycle_pos[0]) {
            return 0.0;
        }
        if i64::from(sample) >= i64::from(self.cycle_pos[n]) {
            return n as f32;
        }
        let sample = sample as u32;
        let g = self.cycle_pos.partition_point(|&p| p <= sample) - 1;
        let start = self.cycle_pos[g] as f32;
        let end = self.cycle_pos[g + 1] as f32;
        (sample as f32 - start) / (end - start) + g as f32
    }

    /// Samples per cycle at fractional cycle index `cycle`: the central-difference period at each cycle
    /// boundary, interpolated linearly in between. 0 without cycles.
    pub fn cycle_period(&self, cycle: f32) -> f32 {
        let n = self.cycle_count() as i32;
        if n < 1 {
            return 0.0;
        }
        if n < 2 {
            return self.span(0, 1);
        }
        let i = round::floor(cycle);
        if i < 1 {
            let start = self.span(0, 1);
            let end = self.span(0, 2) * 0.5;
            return start + cycle.max(0.0) * (end - start);
        }
        if i >= n - 1 {
            let last = n as usize;
            let start = self.span(last - 2, last) * 0.5;
            let end = self.span(last - 1, last);
            let i = i.min(n - 1);
            return start + (cycle.min(n as f32) - i as f32) * (end - start);
        }
        let i = i as usize;
        let start = self.span(i - 1, i + 1) * 0.5;
        let end = self.span(i, i + 2) * 0.5;
        start + (cycle - i as f32) * (end - start)
    }

    /// Splits a requested frequency for a loop that cannot go below its minimum: the frequency to give the
    /// synthesiser (not below `min_frequency`) and the ratio by which the caller scales the playback speed
    /// (below 1 under the minimum, else 1).
    pub fn clamp_frequency(&self, frequency: f32) -> (f32, f32) {
        if frequency >= self.min_frequency || self.min_frequency <= 0.0 || frequency.is_nan() {
            return (frequency, 1.0);
        }
        (self.min_frequency, frequency / self.min_frequency)
    }

    fn freq_at(&self, i: usize) -> i32 {
        self.freq_pos[i] as i32
    }

    fn cycle_at(&self, i: usize) -> i32 {
        self.cycle_pos[i] as i32
    }

    /// `cycle_pos[b] - cycle_pos[a]` as a float.
    fn span(&self, a: usize, b: usize) -> f32 {
        (self.cycle_pos[b] as i64 - self.cycle_pos[a] as i64) as f32
    }
}

fn word(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

fn read_u32s(bytes: &[u8]) -> Vec<u32> {
    bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect()
}
