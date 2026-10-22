//! The unreliable frames of protocol.md section 6 besides `INPUT` (which is [`crate::input`]): `SNAPSHOT`, `PING` and
//! `PONG`, and the snapshot's global block. Shared by the server, which writes them, and the client session, which reads
//! them. The connection layer writes the frame type and length around each payload (ADR 0009), so the type byte is not
//! repeated inside.
//!
//! - [`ping`]: `PING` (seq 8, client time 32) and `PONG` (seq 8, client time 32, receive tick 32 + fraction 8, send
//!   tick 32 + fraction 8), most significant bit first. A PONG tick is in 1/256 ticks.
//! - [`GlobalBlock`]: the 96-bit global block of a snapshot (session phase, the receiver's pursuit and race state, the
//!   time of day).
//! - [`SnapshotHeader`] and [`read_snapshot`]: the `SNAPSHOT` payload as built here: `part` 4 and `parts` 4, `flags` 8
//!   (bit 0 own block, bit 1 global block, the rest zero), `ack_input` 16 (low 16 bits), the own block (256 bits, part 0
//!   only), the global block (96 bits, part 0 only), then the `blackbox_net_core::replicate` snapshot body (tick 32,
//!   baseline distance varint, entry count varint, entries in id order), zero padded to the byte. Every part of one
//!   snapshot carries the same tick and baseline distance and a disjoint set of entities in ascending id order; a
//!   single-part snapshot's body is exactly what `ClientState::write_snapshot` writes. [`write_snapshot`] builds one.
//!
//! Decoders are pure functions on `&[u8]` and never panic (`security-gameplay.md` section 1.1): wrong lengths, reserved
//! bits, out-of-range codes and nonzero padding are a [`FrameError`].

mod global;
mod ping;
mod snapshot;

#[cfg(test)]
mod tests;

pub use global::{GLOBAL_BLOCK_BITS, GlobalBlock, PursuitStatus};
pub use ping::{PING_LEN, PONG_LEN, decode_ping, decode_pong, encode_ping, encode_pong};
pub use snapshot::{MAX_PARTS, SNAPSHOT_HEADER_BYTES, SnapshotFrame, SnapshotHeader, read_snapshot, write_snapshot};

/// The frame type of a snapshot (server to client).
pub const FRAME_SNAPSHOT: u8 = 0x02;
/// The frame type of a PING (client to server).
pub const FRAME_PING: u8 = 0x03;
/// The frame type of a PONG (server to client).
pub const FRAME_PONG: u8 = 0x04;

/// Why a frame could not be encoded or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("the payload has the wrong length")]
    BadLength,
    #[error("a value is outside its field")]
    BadValue,
    #[error("a reserved bit is set")]
    Reserved,
    #[error("the part index or count is invalid, or a block is on a part other than 0")]
    BadPart,
    #[error("the padding bits are not zero")]
    BadPadding,
    #[error("an own or global block is malformed")]
    BadBlock,
}
