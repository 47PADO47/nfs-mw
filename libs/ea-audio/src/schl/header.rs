//! The `SCHl` / `PT` header: a platform marker followed by a stream of `tag, length, value` records.

use crate::error::{Error, Result};

/// Platform id used for `GSTR` ("generic stream") headers.
pub const PLATFORM_GENERIC: u16 = 8;
/// Platform id of the PC.
pub const PLATFORM_PC: u16 = 0;

/// The sample codec of a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// EA-XA ADPCM, one mono frame run per channel.
    EaXa,
    /// EA-XA ADPCM, both channels interleaved nibble by nibble in 30-byte frames.
    EaXaStereo,
    /// EA MicroTalk (10:1 or 5:1; the decoder is the same).
    MicroTalk,
    /// Anything else, with its `codec2` value (or the legacy `codec1` value). Not decoded by this crate.
    Other(u32),
}

/// Everything the header says about a stream or bank sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamHeader {
    /// Platform id (`0` = PC, `8` = generic / `GSTR`).
    pub platform: u16,
    /// Header version (tag `0x80`, or the platform default).
    pub version: u8,
    pub channels: u16,
    pub sample_rate: u32,
    /// Samples per channel (tag `0x85`).
    pub sample_count: u32,
    /// Tag `0x86`.
    pub loop_start: Option<u32>,
    /// Tag `0x87`, as stored (the loop ends after this sample).
    pub loop_end: Option<u32>,
    pub codec: Codec,
    /// Data offset of each channel (tags `0x88`, `0x89`, `0x94`, `0x95`, `0xA2`, `0xA3`); bank sounds only.
    pub channel_offsets: [Option<u32>; 6],
    /// Priority (tag `0x06`).
    pub priority: Option<u32>,
    /// Flags (tag `0x8C`).
    pub flags: u32,
    /// True when `SCDl` values are big-endian.
    pub big_endian: bool,
    /// True for the later EA-XA / MicroTalk revision with PCM frames.
    pub pcm_blocks: bool,
}

impl StreamHeader {
    /// The loop as `(start, end)` frames with `end` exclusive, when the header has one.
    pub fn loop_range(&self) -> Option<(u32, u32)> {
        let end = self.loop_end?;
        Some((self.loop_start.unwrap_or(0), end.saturating_add(1)))
    }
}

#[derive(Default)]
struct Raw {
    version: Option<u32>,
    channels: Option<u32>,
    sample_rate: Option<u32>,
    sample_count: u32,
    loop_start: Option<u32>,
    loop_end: Option<u32>,
    codec1: Option<u32>,
    codec2: Option<u32>,
    offsets: [Option<u32>; 6],
    priority: Option<u32>,
    flags: u32,
}

fn platform_defaults(platform: u16) -> Result<(u32, u32)> {
    // (version, sample rate)
    Ok(match platform {
        0 | 3 | 4 => (0, 22050),
        1 | 2 => (0, 22050),
        5 => (1, 22050),
        6 => (2, 24000),
        7 => (2, 24000),
        8 => (2, 48000),
        9 => (3, 44100),
        0x0A => (3, 22050),
        0x0E => (3, 44100),
        0x10 | 0x14 => (3, 32000),
        _ => return Err(Error::BadHeader("unknown platform")),
    })
}

fn is_big_endian(platform: u16) -> bool {
    matches!(platform, 2 | 3 | 4 | 6 | 8 | 9 | 0x0E | 0x10)
}

fn resolve_codec(raw: &Raw, platform: u16) -> Codec {
    let from_id = |id: u32| match id {
        0x03 => Codec::EaXaStereo,
        0x0A => Codec::EaXa,
        0x04 | 0x16 => Codec::MicroTalk,
        other => Codec::Other(other),
    };
    if let Some(codec2) = raw.codec2 {
        return from_id(codec2);
    }
    if let Some(codec1) = raw.codec1 {
        return match (codec1, platform) {
            (0x07, 0 | 3) => Codec::EaXaStereo,
            (0x07, _) => Codec::EaXa,
            (0x09, _) => Codec::MicroTalk,
            (other, _) => Codec::Other(other),
        };
    }
    match platform {
        0 | 3 | 8 | 9 | 0x0A | 0x0E => Codec::EaXa,
        1 => Codec::Other(0x05),
        2 => Codec::Other(0x06),
        _ => Codec::Other(0xFF),
    }
}

/// Read one tag value: length byte, then that many bytes big-endian. Returns the value and the new position.
fn read_value(data: &[u8], at: usize) -> Result<(u32, usize)> {
    let truncated = Error::Truncated { offset: at as u64, needed: 1 };
    let len = *data.get(at).ok_or(truncated.clone())? as usize;
    let body = at + 1;
    if len == 0xFF {
        let size = crate::bytes::u32_be(data, body)? as usize;
        return Ok((0, body.saturating_add(4).saturating_add(size)));
    }
    let bytes = data.get(body..body + len).ok_or(truncated)?;
    if len > 4 {
        return Ok((0, body + len));
    }
    let value = bytes.iter().fold(0u32, |acc, &b| (acc << 8) | b as u32);
    Ok((value, body + len))
}

fn apply(raw: &mut Raw, tag: u8, value: u32) {
    match tag {
        0x06 => raw.priority = Some(value),
        0x80 => raw.version = Some(value),
        0x82 => raw.channels = Some(value),
        0x83 => raw.codec1 = Some(value),
        0x84 => raw.sample_rate = Some(value),
        0x85 => raw.sample_count = value,
        0x86 => raw.loop_start = Some(value),
        0x87 => raw.loop_end = Some(value),
        0x88 => raw.offsets[0] = Some(value),
        0x89 => raw.offsets[1] = Some(value),
        0x94 => raw.offsets[2] = Some(value),
        0x95 => raw.offsets[3] = Some(value),
        0xA2 => raw.offsets[4] = Some(value),
        0xA3 => raw.offsets[5] = Some(value),
        0x8C => raw.flags = value,
        0xA0 => raw.codec2 = Some(value),
        _ => {}
    }
}

/// Parse a header that starts at its platform marker (`PT` + u16, or `GSTR` + 4 bytes). `data` may run on past the
/// header; parsing stops at the `0xFF` end tag.
pub fn parse(data: &[u8]) -> Result<StreamHeader> {
    let (platform, mut at) = match data.get(..4) {
        Some(b"GSTR") => (PLATFORM_GENERIC, 8),
        Some([b'P', b'T', lo, hi]) => (u16::from_le_bytes([*lo, *hi]), 4),
        Some(_) => return Err(Error::BadMagic { expected: "PT or GSTR" }),
        None => return Err(Error::Truncated { offset: 0, needed: 4 }),
    };
    let mut raw = Raw::default();
    loop {
        let tag = *data.get(at).ok_or(Error::Truncated { offset: at as u64, needed: 1 })?;
        at += 1;
        if tag == 0xFF || tag == 0xFE {
            break;
        }
        if tag == 0xFC || tag == 0xFD {
            continue;
        }
        let (value, next) = read_value(data, at)?;
        apply(&mut raw, tag, value);
        at = next;
    }
    finish(raw, platform)
}

fn finish(raw: Raw, platform: u16) -> Result<StreamHeader> {
    let (default_version, default_rate) = platform_defaults(platform)?;
    let version = raw.version.unwrap_or(default_version);
    let channels = raw.channels.unwrap_or(1).max(1);
    if channels > 6 {
        return Err(Error::BadHeader("more than 6 channels"));
    }
    let pcm_blocks = version == 3 || (version == 2 && matches!(platform, 0 | 3 | 8));
    Ok(StreamHeader {
        platform,
        version: version as u8,
        channels: channels as u16,
        sample_rate: raw.sample_rate.filter(|&r| r != 0).unwrap_or(default_rate),
        sample_count: raw.sample_count,
        loop_start: raw.loop_start,
        loop_end: raw.loop_end,
        codec: resolve_codec(&raw, platform),
        channel_offsets: raw.offsets,
        priority: raw.priority,
        flags: raw.flags,
        big_endian: is_big_endian(platform),
        pcm_blocks,
    })
}
