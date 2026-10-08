//! `SCHl` block streams: a header block, a block count, audio blocks and an end block.
//!
//! Three ways in, from simplest to most economical:
//! - [`decode_stream`] decodes a complete stream held in a slice.
//! - [`Stream`] parses the header and lets you walk the blocks yourself.
//! - [`StreamReader`] decodes one block at a time from a [`crate::ReadAt`] source.
//!
//! Layout: `docs/specs/audio-containers.md` in the repository.

mod blocks;
mod decode;
pub mod header;
mod reader;

pub use blocks::{BLOCK_HEADER, Block, Blocks, TAG_COUNT, TAG_DATA, TAG_END, TAG_HEADER};
pub use decode::StreamDecoder;
pub use header::{Codec, StreamHeader};
pub use reader::{StreamReader, extent};

use crate::error::{Error, Result};
use crate::pcm::Pcm;

/// A parsed stream held in memory: the header and the blocks that follow it.
pub struct Stream<'a> {
    header: StreamHeader,
    rest: &'a [u8],
}

impl<'a> Stream<'a> {
    /// Parse the `SCHl` block at the start of `data`. `data` may continue past the end of the stream.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let mut blocks = Blocks::new(data);
        let first = blocks.next().ok_or(Error::Truncated { offset: 0, needed: BLOCK_HEADER })??;
        if first.tag != TAG_HEADER {
            return Err(Error::BadMagic { expected: "SCHl" });
        }
        Ok(Self { header: header::parse(first.payload)?, rest: &data[blocks.position()..] })
    }

    /// The stream's header.
    pub fn header(&self) -> &StreamHeader {
        &self.header
    }

    /// The blocks after the header, ending with `SCEl`.
    pub fn blocks(&self) -> Blocks<'a> {
        Blocks::new(self.rest)
    }

    /// Decode the whole stream.
    pub fn decode(&self) -> Result<Pcm> {
        let mut decoder = StreamDecoder::new(&self.header)?;
        let mut samples = Vec::new();
        for block in self.blocks() {
            let block = block?;
            if block.tag == TAG_DATA {
                decoder.decode_block(block.payload, &mut samples)?;
            }
        }
        let h = &self.header;
        Ok(Pcm { sample_rate: h.sample_rate, channels: h.channels, samples, loop_range: h.loop_range() })
    }
}

/// Decode the stream at the start of `data`.
pub fn decode_stream(data: &[u8]) -> Result<Pcm> {
    Stream::parse(data)?.decode()
}
