//! The tables of a `Gnsu` file together with its decoded audio.

use crate::tables::GinsuTables;
use crate::{Error, Result};

/// Everything the synthesiser reads: tables and the mono recording as `f32` in -1 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct GinsuData {
    tables: GinsuTables,
    pcm: Vec<f32>,
}

impl GinsuData {
    /// Pairs tables with decoded audio. `pcm` must hold exactly `sample_count` samples.
    pub fn new(tables: GinsuTables, pcm: Vec<f32>) -> Result<Self> {
        let expected = tables.sample_count() as usize;
        if pcm.len() != expected {
            return Err(Error::SampleCountMismatch { expected, actual: pcm.len() });
        }
        Ok(Self { tables, pcm })
    }

    /// As [`Self::new`] for 16-bit samples (scaled by 1/32768).
    pub fn from_i16(tables: GinsuTables, pcm: &[i16]) -> Result<Self> {
        Self::new(tables, pcm.iter().map(|&s| f32::from(s) / 32768.0).collect())
    }

    pub fn tables(&self) -> &GinsuTables {
        &self.tables
    }

    /// The decoded recording.
    pub fn pcm(&self) -> &[f32] {
        &self.pcm
    }

    /// Copies `out.len()` samples from sample `start`. Positions outside the recording read as silence.
    pub fn read(&self, start: i32, out: &mut [f32]) {
        for (i, slot) in out.iter_mut().enumerate() {
            let at = i64::from(start) + i as i64;
            *slot = match usize::try_from(at) {
                Ok(at) => self.pcm.get(at).copied().unwrap_or(0.0),
                Err(_) => 0.0,
            };
        }
    }
}
