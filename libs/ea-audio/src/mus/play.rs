//! Reading a chain of streams as one continuous run of audio.

use super::Mpf;
use super::chain::{Chain, Segment};
use crate::error::{Error, Result};
use crate::schl::StreamReader;
use crate::source::ReadAt;

/// Decodes the streams of a [`Chain`] one after the other, a block at a time, from a [`ReadAt`] source such as
/// the `.mus` file. The blocks of consecutive streams follow each other without a gap, so the output is the
/// whole track as one run of samples. Nothing is kept in memory but the block being decoded.
pub struct ChainReader<'a, S: ReadAt + ?Sized> {
    mpf: &'a Mpf,
    source: &'a S,
    segments: &'a [Segment],
    /// Index in `segments` of the stream being read.
    index: usize,
    current: Option<StreamReader<'a, S>>,
    sample_rate: u32,
    channels: u16,
}

impl<'a, S: ReadAt + ?Sized> ChainReader<'a, S> {
    /// Open the first stream of `chain`. An empty chain is an error.
    pub fn new(mpf: &'a Mpf, source: &'a S, chain: &'a Chain) -> Result<Self> {
        let first = chain.segments.first().ok_or(Error::Corrupt("a chain with no streams"))?;
        let reader = mpf.open(source, first.stream)?;
        let (sample_rate, channels) = (reader.header().sample_rate, reader.header().channels);
        Ok(Self { mpf, source, segments: &chain.segments, index: 0, current: Some(reader), sample_rate, channels })
    }

    /// Sample rate shared by every stream of the chain.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channels of the interleaved output, shared by every stream of the chain.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The segment whose stream the last block came from.
    pub fn segment(&self) -> &Segment {
        &self.segments[self.index]
    }

    /// Index of [`segment`](Self::segment) in the chain.
    pub fn segment_index(&self) -> usize {
        self.index
    }

    /// Decode the next block and append its interleaved samples to `out`, moving on to the next stream when the
    /// current one ends. Returns the frames appended, or `None` after the last block of the last stream.
    pub fn next_chunk(&mut self, out: &mut Vec<i16>) -> Result<Option<usize>> {
        loop {
            let Some(reader) = self.current.as_mut() else { return Ok(None) };
            if let Some(frames) = reader.next_chunk(out)? {
                return Ok(Some(frames));
            }
            self.advance()?;
        }
    }

    /// Open the stream after the current one, or finish.
    fn advance(&mut self) -> Result<()> {
        self.current = None;
        let Some(next) = self.segments.get(self.index + 1) else { return Ok(()) };
        let reader = self.mpf.open(self.source, next.stream)?;
        if (reader.header().sample_rate, reader.header().channels) != (self.sample_rate, self.channels) {
            return Err(Error::Corrupt("the streams of a chain differ in rate or channels"));
        }
        self.index += 1;
        self.current = Some(reader);
        Ok(())
    }
}
