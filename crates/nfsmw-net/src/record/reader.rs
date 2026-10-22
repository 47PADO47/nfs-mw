//! The [`Replay`] reader: header, frames with resync over damage, and seeking through the index or by a scan.

use nfsmw_sim::Tick;

use super::crc::crc32;
use super::frame::{FrameKind, next_sync, read_frame};
use super::header::{ReplayError, ReplayHeader};
use super::writer::{INDEX_MAGIC, MAX_INDEX_ENTRIES, TRAILER_LEN};

/// One intact frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    pub kind: FrameKind,
    /// A key snapshot.
    pub key: bool,
    pub tick: Tick,
    pub payload: &'a [u8],
    /// Where the frame starts in the file.
    pub offset: usize,
}

/// The intact frames of a replay in file order, skipping damage and the index frame.
#[derive(Debug, Clone)]
pub struct Frames<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
    damaged: usize,
}

impl<'a> Frames<'a> {
    /// Bytes skipped so far because they were not part of an intact frame (damage, a cut end).
    pub fn damaged(&self) -> usize {
        self.damaged
    }

    fn skip_to_next_sync(&mut self, from: usize) {
        let next = next_sync(self.bytes, from, self.end).unwrap_or(self.end);
        self.damaged += next - self.pos;
        self.pos = next;
    }
}

impl<'a> Iterator for Frames<'a> {
    type Item = Frame<'a>;

    fn next(&mut self) -> Option<Frame<'a>> {
        while self.pos < self.end {
            let (frame, after) = match read_frame(self.bytes, self.pos, self.end) {
                Some(found) => found,
                None => {
                    self.skip_to_next_sync(self.pos + 1);
                    continue;
                }
            };
            let offset = self.pos;
            self.pos = after;
            if frame.kind == FrameKind::Index {
                continue;
            }
            return Some(Frame { kind: frame.kind, key: frame.key, tick: frame.tick, payload: frame.payload, offset });
        }
        None
    }
}

/// A replay file in memory.
#[derive(Debug, Clone)]
pub struct Replay<'a> {
    bytes: &'a [u8],
    header: ReplayHeader,
    body_start: usize,
    body_end: usize,
    index: Option<Vec<(Tick, usize)>>,
}

impl<'a> Replay<'a> {
    /// Opens `bytes`. Only a damaged header is an error; a missing or damaged index means seeks scan.
    pub fn open(bytes: &'a [u8]) -> Result<Self, ReplayError> {
        let (header, body_start) = ReplayHeader::decode(bytes)?;
        let mut replay = Self { bytes, header, body_start, body_end: bytes.len(), index: None };
        if let Some((index, index_offset)) = replay.read_index() {
            replay.index = Some(index);
            replay.body_end = index_offset;
        }
        Ok(replay)
    }

    pub fn header(&self) -> &ReplayHeader {
        &self.header
    }

    /// Whether the file has an intact index.
    pub fn has_index(&self) -> bool {
        self.index.is_some()
    }

    /// Every intact frame from the first.
    pub fn frames(&self) -> Frames<'a> {
        self.frames_from(self.body_start)
    }

    /// Every intact frame from `offset` (a [`Replay::seek`] result).
    pub fn frames_from(&self, offset: usize) -> Frames<'a> {
        let pos = offset.clamp(self.body_start, self.body_end);
        Frames { bytes: self.bytes, pos, end: self.body_end, damaged: 0 }
    }

    /// The offset of the last key snapshot at or before `tick`; with no such snapshot, the first frame's offset. The
    /// index (when intact and pointing at an intact key snapshot) gives the start of a scan of at most one stride;
    /// without it the scan starts at the first frame.
    pub fn seek(&self, tick: Tick) -> usize {
        let indexed = self.index.as_ref().and_then(|index| {
            let count = index.partition_point(|(at, _)| *at <= tick);
            let (_, offset) = *index.get(count.checked_sub(1)?)?;
            let (frame, _) = read_frame(self.bytes, offset, self.body_end)?;
            (frame.kind == FrameKind::Snapshot && frame.key && frame.tick <= tick).then_some(offset)
        });
        let start = indexed.unwrap_or(self.body_start);
        let mut best = start;
        for frame in self.frames_from(start) {
            if frame.tick > tick {
                break;
            }
            if frame.kind == FrameKind::Snapshot && frame.key {
                best = frame.offset;
            }
        }
        best
    }

    /// The index and its frame's offset, when the trailer and the index frame are intact and consistent.
    fn read_index(&self) -> Option<(Vec<(Tick, usize)>, usize)> {
        let len = self.bytes.len();
        let trailer_start = len.checked_sub(TRAILER_LEN).filter(|start| *start >= self.body_start)?;
        let trailer = &self.bytes[trailer_start..];
        if trailer[..8] != INDEX_MAGIC {
            return None;
        }
        let stored = u32::from_le_bytes([trailer[16], trailer[17], trailer[18], trailer[19]]);
        if crc32(&trailer[..16]) != stored {
            return None;
        }
        let mut offset_bytes = [0u8; 8];
        offset_bytes.copy_from_slice(&trailer[8..16]);
        let index_offset = usize::try_from(u64::from_le_bytes(offset_bytes)).ok()?;
        if index_offset < self.body_start {
            return None;
        }
        let (frame, after) = read_frame(self.bytes, index_offset, trailer_start)?;
        if frame.kind != FrameKind::Index || after != trailer_start {
            return None;
        }
        let index = parse_index(frame.payload, self.body_start, index_offset)?;
        Some((index, index_offset))
    }
}

/// The entries of an index payload; `None` unless every entry lies in the body and the ticks never go back.
fn parse_index(payload: &[u8], body_start: usize, body_end: usize) -> Option<Vec<(Tick, usize)>> {
    let count = usize::try_from(u32::from_le_bytes(payload.get(..4)?.try_into().ok()?)).ok()?;
    if count > MAX_INDEX_ENTRIES || payload.len() != 8 + count * 12 {
        return None;
    }
    let mut entries = Vec::with_capacity(count);
    for entry in payload[8..].as_chunks::<12>().0 {
        let tick = u32::from_le_bytes(entry[..4].try_into().ok()?);
        let offset = usize::try_from(u64::from_le_bytes(entry[4..].try_into().ok()?)).ok()?;
        if offset < body_start || offset >= body_end {
            return None;
        }
        if entries.last().is_some_and(|(last, _): &(Tick, usize)| tick < *last) {
            return None;
        }
        entries.push((tick, offset));
    }
    Some(entries)
}
