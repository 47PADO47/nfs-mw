//! Parsing the `.mpf` file (version 5).

use crate::bytes::{u8_at, u16_le, u32_be, u32_le};
use crate::error::{Error, Result};
use crate::pcm::Pcm;
use crate::schl::{StreamReader, extent};
use crate::source::ReadAt;

/// Unit of the stream offsets in the sample table.
const OFFSET_UNIT: u64 = 0x80;

/// A track: a run of consecutive entries of the stream table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    /// Index of the track's first stream.
    pub first_stream: u32,
    /// Number of RAM sub-banks; 0 means the track's streams are streamed from the `.mus` file.
    pub sub_banks: u16,
    /// Checksum of the `.mus` file the track belongs to (big-endian in the file; equals its first 4 bytes).
    pub checksum: u32,
}

/// One entry of the stream table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusStream {
    /// Raw first word of the entry (the offset in 0x80 units, or bank and sound index for RAM tracks).
    pub raw: u32,
    /// Byte offset of the `SCHl` block in the `.mus` file (`raw * 0x80`). Meaningful for streamed tracks.
    pub offset: u64,
    /// Duration in milliseconds, as stored.
    pub duration_ms: u32,
    /// Index of the track this stream belongs to.
    pub track: usize,
}

/// A parsed `.mpf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mpf {
    pub version: u8,
    pub sub_version: u8,
    pub tracks: Vec<Track>,
    pub streams: Vec<MusStream>,
}

impl Mpf {
    /// Parse a version 5 map file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.get(..4) != Some(b"xDFP") {
            return Err(Error::BadMagic { expected: "xDFP (little-endian PFDx)" });
        }
        let version = u8_at(data, 4)?;
        if version != 5 {
            return Err(Error::Unsupported("mpf version other than 5"));
        }
        let sub_version = u8_at(data, 5)?;
        let track_count = u8_at(data, 0x0D)? as usize;
        let tracks_table = u32_le(data, 0x2C)? as usize;
        let samples_table = u32_le(data, 0x34)? as usize;
        let samples_end = u32_le(data, 0x38)? as usize;
        if samples_end < samples_table || samples_end > data.len() {
            return Err(Error::Corrupt("stream table outside the file"));
        }
        let tracks = (0..track_count)
            .map(|i| {
                let entry = u32_le(data, tracks_table + 4 * i)? as usize * 4;
                Ok(Track {
                    first_stream: u32_le(data, entry)?,
                    sub_banks: u16_le(data, entry + 4)?,
                    checksum: u32_be(data, entry + 8)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let count = (samples_end - samples_table) / 8;
        let streams = (0..count)
            .map(|k| {
                let raw = u32_le(data, samples_table + 8 * k)?;
                Ok(MusStream {
                    raw,
                    offset: raw as u64 * OFFSET_UNIT,
                    duration_ms: u32_le(data, samples_table + 8 * k + 4)?,
                    track: track_of(&tracks, k),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { version, sub_version, tracks, streams })
    }

    /// True when the first four bytes of the `.mus` file match the first track's checksum.
    pub fn matches_mus(&self, mus_head: &[u8]) -> bool {
        let Some(track) = self.tracks.first() else { return false };
        mus_head.get(..4).is_some_and(|h| h == track.checksum.to_be_bytes())
    }

    /// Total length of all streams in seconds, from the stored durations.
    pub fn total_secs(&self) -> f64 {
        self.streams.iter().map(|s| s.duration_ms as f64).sum::<f64>() / 1000.0
    }

    /// Length in bytes of stream `index` in the `.mus`, found by walking its block prefixes.
    pub fn stream_len<S: ReadAt + ?Sized>(&self, mus: &S, index: usize) -> Result<u64> {
        let stream = self.streams.get(index).ok_or(Error::NoSuchEntry(index))?;
        extent(mus, stream.offset)
    }

    /// Open stream `index` for block-by-block decoding.
    pub fn open<'s, S: ReadAt + ?Sized>(&self, mus: &'s S, index: usize) -> Result<StreamReader<'s, S>> {
        let stream = self.streams.get(index).ok_or(Error::NoSuchEntry(index))?;
        StreamReader::open(mus, stream.offset)
    }

    /// Decode stream `index` completely.
    pub fn decode<S: ReadAt + ?Sized>(&self, mus: &S, index: usize) -> Result<Pcm> {
        self.open(mus, index)?.read_all()
    }
}

/// The track of stream `k`: the last non-empty track that starts at or before it.
fn track_of(tracks: &[Track], k: usize) -> usize {
    tracks
        .iter()
        .enumerate()
        .rev()
        .find(|(i, t)| (t.first_stream != 0 || *i == 0) && t.first_stream as usize <= k)
        .map_or(0, |(i, _)| i)
}
