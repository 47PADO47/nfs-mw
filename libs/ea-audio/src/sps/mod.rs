//! `.SPS` streams: EA Audio Core's container, as the Chameleon engine's NFS: Most Wanted (2012) uses it.
//!
//! A stream is an `H` chunk (the [`Header`]) followed by `D` chunks, each `D`, a 24-bit size that includes the 8-byte
//! chunk header, a 32-bit sample count and the frames. Chunks are big endian. A stream of more than two channels is
//! made of *cores*, one per stereo pair (and a mono core for an odd last channel): inside a chunk the cores' frames
//! alternate, core 0, core 1, ... and again. Only the Layer III codec ([`Codec::EaLayer3`]) is read: [`to_mp3`] repacks
//! each core's frames as an ordinary MPEG-1 Layer III stream, which any MP3 decoder turns into audio. The crate does not
//! decode MP3 itself.

mod bits;
mod header;
mod layer3;

#[cfg(test)]
mod tests;

pub use header::{Codec, Header};

use crate::error::{Error, Result};

/// One core of a stream: a stereo or mono pair of channels, as MPEG-1 Layer III frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Core {
    /// The channels this core carries: 2, or 1 for a mono core.
    pub channels: u16,
    /// The MPEG-1 Layer III frames, back to back.
    pub bytes: Vec<u8>,
    /// How many MP3 frames `bytes` holds.
    pub frames: usize,
}

/// A Layer III stream repacked as MPEG-1 frames, one stream per core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mp3 {
    pub header: Header,
    /// The cores in channel order: core 0 carries channels 0 and 1, core 1 channels 2 and 3, and so on.
    pub cores: Vec<Core>,
}

/// Read the `H` chunk of an SPS stream.
pub fn read_header(data: &[u8]) -> Result<Header> {
    header::read(data).map(|(header, _)| header)
}

/// Repack the Layer III frames of an SPS stream as MPEG-1 Layer III streams, one per core.
pub fn to_mp3(data: &[u8]) -> Result<Mp3> {
    let (header, mut at) = header::read(data)?;
    if let Codec::Other(number) = header.codec {
        return Err(Error::UnsupportedCodec(u32::from(number)));
    }
    let core_count = usize::from(header.channels).div_ceil(2);
    let mut cores: Vec<layer3::Repacker> = (0..core_count).map(|_| layer3::Repacker::default()).collect();
    while at < data.len() {
        let size = header::chunk_size(&data[at..])?;
        let chunk = data.get(at..at + size).ok_or(Error::Truncated { offset: at as u64, needed: size })?;
        at += size;
        if chunk[0] != b'D' {
            continue;
        }
        let payload = chunk.get(8..).ok_or(Error::Corrupt("a data chunk shorter than its header"))?;
        let mut used = 0;
        let mut slot = 0;
        while used < payload.len() {
            let (granule, len) = layer3::read_granule(&payload[used..])?;
            used += len;
            cores[slot % core_count].push_granule(granule)?;
            slot += 1;
        }
    }
    let mut out = Vec::with_capacity(core_count);
    for (at, mut core) in cores.into_iter().enumerate() {
        core.finish()?;
        let channels = (usize::from(header.channels) - at * 2).min(2) as u16;
        out.push(Core { channels, bytes: core.bytes, frames: core.frames });
    }
    Ok(Mp3 { header, cores: out })
}
