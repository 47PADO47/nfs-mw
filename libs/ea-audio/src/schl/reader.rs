//! Reading a stream block by block from a [`ReadAt`] source, so a 500 MB music file never has to be in memory.

use super::blocks::{BLOCK_HEADER, TAG_DATA, TAG_END, TAG_HEADER, parse_prefix};
use super::decode::StreamDecoder;
use super::header::{self, StreamHeader};
use crate::error::{Error, Result};
use crate::pcm::Pcm;
use crate::source::{ReadAt, read_array, read_vec};

/// A forward-only reader that decodes one `SCDl` block at a time.
pub struct StreamReader<'s, S: ReadAt + ?Sized> {
    src: &'s S,
    pos: u64,
    decoder: StreamDecoder,
    finished: bool,
}

impl<'s, S: ReadAt + ?Sized> StreamReader<'s, S> {
    /// Open the stream whose `SCHl` block starts at `offset`.
    pub fn open(src: &'s S, offset: u64) -> Result<Self> {
        let (tag, size) = parse_prefix(&read_array(src, offset)?)?;
        if tag != TAG_HEADER {
            return Err(Error::BadMagic { expected: "SCHl" });
        }
        let body = read_vec(src, offset + BLOCK_HEADER as u64, size - BLOCK_HEADER)?;
        let header = header::parse(&body)?;
        Ok(Self { src, pos: offset + size as u64, decoder: StreamDecoder::new(&header)?, finished: false })
    }

    /// The stream's header.
    pub fn header(&self) -> &StreamHeader {
        self.decoder.header()
    }

    /// Position of the next unread block.
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// Decode the next audio block and append its interleaved samples to `out`. Returns the number of frames
    /// appended, or `None` once the end block has been read.
    pub fn next_chunk(&mut self, out: &mut Vec<i16>) -> Result<Option<usize>> {
        while !self.finished {
            let (tag, size) = parse_prefix(&read_array(self.src, self.pos)?)?;
            if tag == TAG_END {
                self.finished = true;
                break;
            }
            let at = self.pos;
            self.pos += size as u64;
            if tag != TAG_DATA {
                continue;
            }
            let buf = read_vec(self.src, at + BLOCK_HEADER as u64, size - BLOCK_HEADER)?;
            return self.decoder.decode_block(&buf, out).map(Some);
        }
        Ok(None)
    }

    /// Decode everything that is left into one [`Pcm`].
    pub fn read_all(mut self) -> Result<Pcm> {
        let mut samples = Vec::new();
        while self.next_chunk(&mut samples)?.is_some() {}
        let h = self.decoder.header();
        Ok(Pcm { sample_rate: h.sample_rate, channels: h.channels, samples, loop_range: h.loop_range() })
    }
}

/// Length in bytes of the stream whose `SCHl` block starts at `offset`: the offset of the end of its `SCEl` block
/// minus `offset`. Only block prefixes are read.
pub fn extent<S: ReadAt + ?Sized>(src: &S, offset: u64) -> Result<u64> {
    let (tag, _) = parse_prefix(&read_array(src, offset)?)?;
    if tag != TAG_HEADER {
        return Err(Error::BadMagic { expected: "SCHl" });
    }
    let mut pos = offset;
    loop {
        let (tag, size) = parse_prefix(&read_array(src, pos)?)?;
        pos += size as u64;
        if tag == TAG_END {
            return Ok(pos - offset);
        }
    }
}
