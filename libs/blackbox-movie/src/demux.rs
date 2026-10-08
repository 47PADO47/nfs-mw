//! The block walker.

use std::io::Read;

use crate::error::MovieError;
use crate::header::MovieHeader;
use crate::packet::{AudioPacket, Packet, VideoPacket};

/// Size of a block header: `char[4] tag` plus `u32 size` (little-endian, includes the header).
const BLOCK_HEADER_LEN: u64 = 8;

/// A raw block.
struct Block {
    tag: [u8; 4],
    payload: Vec<u8>,
}

/// Demuxer for the EA VP6 movie container.
///
/// Reads blocks one by one from any [`Read`] (a `&[u8]` works), so a movie never has to be loaded
/// whole. The `MVhd` block, and a directly following `SCHl` block, are read by [`Demuxer::new`];
/// everything after is returned by [`Demuxer::next_packet`] or by iterating.
pub struct Demuxer<R> {
    reader: R,
    header: MovieHeader,
    audio_header: Option<Vec<u8>>,
    pending: Option<Block>,
    video_index: u32,
    audio_index: u32,
    audio_samples: u64,
    failed: bool,
}

impl<R: Read> Demuxer<R> {
    /// Reads the header blocks. Fails if the data does not start with `MVhd`.
    pub fn new(mut reader: R) -> Result<Self, MovieError> {
        let first = read_block(&mut reader)?;
        let first = first.ok_or(MovieError::Truncated { tag: *b"MVhd", wanted: BLOCK_HEADER_LEN, got: 0 })?;
        if &first.tag != b"MVhd" {
            return Err(MovieError::NotAMovie(first.tag));
        }
        let header = MovieHeader::parse(&first.payload)?;
        let mut demuxer = Self {
            reader,
            header,
            audio_header: None,
            pending: None,
            video_index: 0,
            audio_index: 0,
            audio_samples: 0,
            failed: false,
        };
        let Some(second) = read_block(&mut demuxer.reader)? else {
            return Ok(demuxer);
        };
        if &second.tag == b"SCHl" {
            demuxer.audio_header = Some(second.payload);
            return Ok(demuxer);
        }
        demuxer.pending = Some(second);
        Ok(demuxer)
    }

    /// The `MVhd` header.
    pub fn header(&self) -> &MovieHeader {
        &self.header
    }

    /// The payload of the `SCHl` block (platform marker and tag stream), for the audio decoder.
    /// `None` if the movie has no audio header right after `MVhd`.
    pub fn audio_header(&self) -> Option<&[u8]> {
        self.audio_header.as_deref()
    }

    /// Returns the next block as a packet, or `None` at the end of the stream.
    pub fn next_packet(&mut self) -> Result<Option<Packet>, MovieError> {
        let block = match self.pending.take() {
            Some(block) => block,
            None => match read_block(&mut self.reader)? {
                Some(block) => block,
                None => return Ok(None),
            },
        };
        self.classify(block).map(Some)
    }

    fn classify(&mut self, block: Block) -> Result<Packet, MovieError> {
        match &block.tag {
            b"MV0K" | b"MV0F" => Ok(Packet::Video(self.video_packet(block))),
            b"SCDl" => self.audio_packet(block).map(Packet::Audio),
            b"SCCl" => {
                let count = be_u32(&block, 0)?;
                Ok(Packet::AudioCount(count))
            }
            b"SCEl" => Ok(Packet::AudioEnd),
            _ => Ok(Packet::Unknown { tag: block.tag, data: block.payload }),
        }
    }

    fn video_packet(&mut self, block: Block) -> VideoPacket {
        let index = self.video_index;
        self.video_index += 1;
        VideoPacket { index, key: &block.tag == b"MV0K", time: self.header.frame_time(index), data: block.payload }
    }

    fn audio_packet(&mut self, block: Block) -> Result<AudioPacket, MovieError> {
        let samples = be_u32(&block, 0)? & 0x00FF_FFFF;
        let packet =
            AudioPacket { index: self.audio_index, first_sample: self.audio_samples, samples, data: block.payload };
        self.audio_index += 1;
        self.audio_samples += u64::from(samples);
        Ok(packet)
    }
}

impl<R: Read> Iterator for Demuxer<R> {
    type Item = Result<Packet, MovieError>;

    /// Yields packets until the end; after an error it yields that error once and then stops.
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        match self.next_packet() {
            Ok(packet) => packet.map(Ok),
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}

fn be_u32(block: &Block, at: usize) -> Result<u32, MovieError> {
    let Some(bytes) = block.payload.get(at..at + 4) else {
        return Err(MovieError::ShortPayload { tag: block.tag, len: block.payload.len(), need: at + 4 });
    };
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reads one block. `None` means a clean end of stream at a block boundary.
fn read_block<R: Read>(reader: &mut R) -> Result<Option<Block>, MovieError> {
    let mut head = [0u8; BLOCK_HEADER_LEN as usize];
    let got = read_up_to(reader, &mut head)?;
    if got == 0 {
        return Ok(None);
    }
    let tag = [head[0], head[1], head[2], head[3]];
    if (got as u64) < BLOCK_HEADER_LEN {
        return Err(MovieError::Truncated { tag, wanted: BLOCK_HEADER_LEN, got: got as u64 });
    }
    let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if u64::from(size) < BLOCK_HEADER_LEN {
        return Err(MovieError::BadBlockSize { tag, size });
    }
    let wanted = u64::from(size) - BLOCK_HEADER_LEN;
    // `take` bounds the allocation by what the reader really holds, so a corrupt size cannot
    // make us reserve gigabytes.
    let mut payload = Vec::new();
    let got = reader.by_ref().take(wanted).read_to_end(&mut payload)? as u64;
    if got < wanted {
        return Err(MovieError::Truncated { tag, wanted: u64::from(size), got: got + BLOCK_HEADER_LEN });
    }
    Ok(Some(Block { tag, payload }))
}

/// Like `read_exact`, but returns how many bytes were read when the stream ends early.
fn read_up_to<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<usize, MovieError> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = match reader.read(&mut buf[filled..]) {
            Ok(n) => n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}
