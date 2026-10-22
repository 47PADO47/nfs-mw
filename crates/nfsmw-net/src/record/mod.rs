//! Replay recording (MP-71 server side, MP-72 client side): a file format, a [`Recorder`] that writes it into a
//! bounded buffer the caller drains to disk, and a [`Replay`] reader over a byte slice that seeks by tick and survives
//! damage. No I/O here: the caller owns the file. The stream is the replay (sync-model.md section 6): a server records
//! the snapshot packets it builds and the reliable events it sends; a client records the snapshots it received plus
//! its own inputs and predicted states, for netcode debugging and a personal replay. Telemetry F-33 shares the format.
//!
//! **Header** (little-endian, [`HEADER_LEN`] bytes at format 1): the magic `NFSMWRPL`, the format (u16), the header
//! length (u16, so a later format can append fields), the kind (u8: 0 server, 1 client), the recording player (u8, the
//! client's own id; 0 on a server), the entity schema id (u64), the protocol minor (u16), the sim build (8 bytes), the
//! data id (16 bytes), the start tick (u32), the tick rate (u8), then a CRC-32 of everything before it.
//!
//! **Frames**, back to back after the header: a sync mark `A5 5A`, the kind byte (low 4 bits: 1 snapshot, 2 event, 3
//! input, 4 predicted state, 5 index; bit 7: a key snapshot that needs no baseline), the tick (u32), the payload
//! length (u32, at most [`MAX_FRAME_PAYLOAD`]), the payload, and a CRC-32 over kind, tick, length and payload. Ticks
//! never go back. Payloads are opaque to the format: an encoded snapshot packet, an encoded reliable message, an
//! encoded input frame, or a [`PredictedState`].
//!
//! **Index and trailer**, written by [`Recorder::finish`]: an index frame (count u32, stride u32, then per entry the
//! tick u32 and the absolute offset u64 of a key snapshot) and a 20-byte trailer: the magic `NFSMWIDX`, the index
//! frame's offset (u64) and a CRC-32 of those 16 bytes. Entries are at least one stride apart (1 s at first); when
//! [`MAX_INDEX_ENTRIES`] are reached every other entry is dropped and the stride doubles, so the index stays bounded.
//!
//! **Bounded memory.** The recorder holds at most its `max_pending` bytes until the caller drains them
//! ([`Recorder::take`]); a frame that does not fit is dropped and counted, and the caller should make the next
//! snapshot a key. The reader borrows the bytes and allocates only the index.
//!
//! **Damage.** A file without a valid trailer (a crash, a cut copy) is read by scanning; a frame with a bad sync, a bad
//! length or a bad CRC is skipped up to the next sync mark that starts a valid frame, and the skipped bytes are counted
//! ([`Frames::damaged`]). Only a damaged header makes the file unreadable.

mod client;
mod crc;
mod frame;
mod header;
mod reader;
mod writer;

#[cfg(test)]
mod tests;

pub use client::{PREDICTED_STATE_LEN, PredictedState};
pub use crc::crc32;
pub use frame::{FRAME_OVERHEAD, FrameKind, MAX_FRAME_PAYLOAD, SYNC};
pub use header::{HEADER_LEN, REPLAY_FORMAT, REPLAY_MAGIC, RecordKind, ReplayError, ReplayHeader};
pub use reader::{Frame, Frames, Replay};
pub use writer::{
    DEFAULT_MAX_PENDING, INDEX_MAGIC, INDEX_STRIDE_TICKS, MAX_INDEX_ENTRIES, RecordError, Recorder, TRAILER_LEN,
};
