//! The [`Recorder`]: frames into a bounded buffer the caller drains, the seek index, and the trailer.

use nfsmw_sim::{TICK_HZ, Tick};

use super::client::PredictedState;
use super::crc::crc32;
use super::frame::{FRAME_OVERHEAD, FrameKind, MAX_FRAME_PAYLOAD, RawFrame, write_frame};
use super::header::{RecordKind, ReplayHeader};

/// The magic of the trailer.
pub const INDEX_MAGIC: [u8; 8] = *b"NFSMWIDX";
/// Bytes of the trailer: magic 8, index offset 8, CRC 4.
pub const TRAILER_LEN: usize = 20;
/// Most entries of the seek index.
pub const MAX_INDEX_ENTRIES: usize = 4096;
/// The first stride between index entries: one second.
pub const INDEX_STRIDE_TICKS: u32 = TICK_HZ;
/// The default bound of undrained bytes: 4 MiB.
pub const DEFAULT_MAX_PENDING: usize = 4 * 1024 * 1024;

/// Why a frame was not recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum RecordError {
    /// The undrained bytes would pass the bound; the frame was dropped and counted.
    #[error("the recorder's buffer is full")]
    Full,
    /// A tick earlier than the last frame's.
    #[error("tick {tick} is before the last frame's tick {last}")]
    OutOfOrder { last: Tick, tick: Tick },
    #[error("the payload is larger than a frame can hold")]
    TooLarge,
    /// Inputs and predicted states belong to a client recording.
    #[error("a server recording has no inputs or predicted states")]
    WrongKind,
    #[error("the recording is finished")]
    Finished,
}

/// Writes one replay into memory the caller drains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorder {
    kind: RecordKind,
    pending: Vec<u8>,
    max_pending: usize,
    /// Bytes already handed to the caller.
    drained: u64,
    last_tick: Option<Tick>,
    index: Vec<(Tick, u64)>,
    stride: u32,
    dropped: u64,
    finished: bool,
}

impl Recorder {
    /// A recording that starts with `header` and holds at most `max_pending` undrained bytes (at least the header and
    /// one largest frame).
    pub fn new(header: &ReplayHeader, max_pending: usize) -> Self {
        let mut pending = Vec::new();
        header.encode(&mut pending);
        let floor = pending.len() + FRAME_OVERHEAD + MAX_FRAME_PAYLOAD;
        Self {
            kind: header.kind,
            pending,
            max_pending: max_pending.max(floor),
            drained: 0,
            last_tick: None,
            index: Vec::new(),
            stride: INDEX_STRIDE_TICKS,
            dropped: 0,
            finished: false,
        }
    }

    /// Records a snapshot packet; `key` when it needs no baseline (only key snapshots are seek points).
    pub fn snapshot(&mut self, tick: Tick, payload: &[u8], key: bool) -> Result<(), RecordError> {
        let offset = self.push(FrameKind::Snapshot, key, tick, payload)?;
        if key {
            self.index_key(tick, offset);
        }
        Ok(())
    }

    /// Records an encoded reliable event message.
    pub fn event(&mut self, tick: Tick, payload: &[u8]) -> Result<(), RecordError> {
        self.push(FrameKind::Event, false, tick, payload).map(|_| ())
    }

    /// Records the client's own encoded input frame (client recordings only).
    pub fn input(&mut self, tick: Tick, payload: &[u8]) -> Result<(), RecordError> {
        if self.kind != RecordKind::Client {
            return Err(RecordError::WrongKind);
        }
        self.push(FrameKind::Input, false, tick, payload).map(|_| ())
    }

    /// Records the client's predicted own state (client recordings only).
    pub fn predicted(&mut self, tick: Tick, state: &PredictedState) -> Result<(), RecordError> {
        if self.kind != RecordKind::Client {
            return Err(RecordError::WrongKind);
        }
        let mut payload = Vec::with_capacity(super::client::PREDICTED_STATE_LEN);
        state.encode(&mut payload);
        self.push(FrameKind::Predicted, false, tick, &payload).map(|_| ())
    }

    /// Hands the recorded bytes to the caller, who appends them to the file.
    pub fn take(&mut self) -> Vec<u8> {
        self.drained += self.pending.len() as u64;
        std::mem::take(&mut self.pending)
    }

    /// Undrained bytes.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Frames dropped because the buffer was full.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The index entries so far: tick and absolute offset of key snapshots.
    pub fn index(&self) -> &[(Tick, u64)] {
        &self.index
    }

    /// Ends the recording: appends the index frame and the trailer (outside the bound, at most about 48 KiB) and
    /// returns the last bytes to write. Later frames are refused.
    pub fn finish(&mut self) -> Result<Vec<u8>, RecordError> {
        if self.finished {
            return Err(RecordError::Finished);
        }
        self.finished = true;
        let mut payload = Vec::with_capacity(8 + self.index.len() * 12);
        payload.extend_from_slice(&(self.index.len() as u32).to_le_bytes());
        payload.extend_from_slice(&self.stride.to_le_bytes());
        for (tick, offset) in &self.index {
            payload.extend_from_slice(&tick.to_le_bytes());
            payload.extend_from_slice(&offset.to_le_bytes());
        }
        let index_offset = self.offset();
        let tick = self.last_tick.unwrap_or(0);
        write_frame(&mut self.pending, &RawFrame { kind: FrameKind::Index, key: false, tick, payload: &payload });
        let trailer_start = self.pending.len();
        self.pending.extend_from_slice(&INDEX_MAGIC);
        self.pending.extend_from_slice(&index_offset.to_le_bytes());
        let crc = crc32(&self.pending[trailer_start..]);
        self.pending.extend_from_slice(&crc.to_le_bytes());
        Ok(self.take())
    }

    /// The absolute offset of the next byte.
    fn offset(&self) -> u64 {
        self.drained + self.pending.len() as u64
    }

    /// Appends one frame; returns its absolute offset.
    fn push(&mut self, kind: FrameKind, key: bool, tick: Tick, payload: &[u8]) -> Result<u64, RecordError> {
        if self.finished {
            return Err(RecordError::Finished);
        }
        if payload.len() > MAX_FRAME_PAYLOAD {
            return Err(RecordError::TooLarge);
        }
        if let Some(last) = self.last_tick
            && tick < last
        {
            return Err(RecordError::OutOfOrder { last, tick });
        }
        if self.pending.len() + FRAME_OVERHEAD + payload.len() > self.max_pending {
            self.dropped += 1;
            return Err(RecordError::Full);
        }
        let offset = self.offset();
        write_frame(&mut self.pending, &RawFrame { kind, key, tick, payload });
        self.last_tick = Some(tick);
        Ok(offset)
    }

    /// Adds a key snapshot to the index when it is a stride past the last entry; thins the index when it is full.
    fn index_key(&mut self, tick: Tick, offset: u64) {
        let due = self.index.last().is_none_or(|(last, _)| tick >= last.saturating_add(self.stride));
        if !due {
            return;
        }
        if self.index.len() >= MAX_INDEX_ENTRIES {
            let mut position = 0usize;
            self.index.retain(|_| {
                let keep = position % 2 == 0;
                position += 1;
                keep
            });
            self.stride = self.stride.saturating_mul(2);
            let still_due = self.index.last().is_none_or(|(last, _)| tick >= last.saturating_add(self.stride));
            if !still_due {
                return;
            }
        }
        self.index.push((tick, offset));
    }
}
