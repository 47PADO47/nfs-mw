//! A sans-IO connection (protocol.md section 5, ADR 0009 sections 3 and 5): it turns bytes into bytes and never touches a
//! socket or a clock. The caller moves datagrams through a [`crate::link::Link`] and passes `now_ms`.
//!
//! **Packet layout** (all multi-byte integers little-endian):
//!
//! | Offset | Size | Field |
//! |---|---|---|
//! | 0 | 1 | flags: bit 7 = 0 (data packet), bits 6..5 = key phase (sealed packets only), bit 4 zero, bits 3..1 = format 0, bit 0 = ack block valid |
//! | 1 | 2 | connection id of the receiver |
//! | 3 | 4 | packet number (starts at 0, +1 per packet) |
//! | 7 | 4 | `ack`: largest packet number received from the peer (0 with `ack_bits` 0 before any) |
//! | 11 | 4 | `ack_bits`: bit `i` set means packet `ack - 1 - i` was received |
//! | 15 | n | frames, at most [`FRAME_SPACE`] bytes |
//! | 15 + n | 0 or 16 | authentication tag of a sealed packet |
//!
//! Flag bit 0 (`FLAG_ACK_VALID`) is set once the sender has received at least one packet. A receiver ignores the ack
//! block while it is clear, so "nothing received yet" is not confused with "packet 0 received". It takes the lowest bit
//! of the format field, which therefore sits in bits 3..1 and is 0 for format 0 (protocol.md section 5.1 records this).
//!
//! **Sealing** ([`Sealer`], `seal.rs`): bytes 0..7 (flags, connection id, packet number) travel in the clear and are the
//! associated data; bytes 7.. (the ack block and the frames) are encrypted and a 16-byte tag follows. [`FRAME_SPACE`]
//! leaves room for the tag, so a sealed packet still fits in [`crate::MAX_DATAGRAM`]. A receiver checks the clear header
//! (handshake bit, connection id) and asks the receive window whether the number is new *before* opening, and records
//! the number only *after* the tag verified, so a forged packet changes nothing and a replayed one is a duplicate.
//! [`Plain`] is the sealer of the in-memory link (no tag); [`Connection::new`] uses it.
//!
//! **Frames** (ADR 0009 section 4): `type` u8, then the payload length as a byte-aligned LEB128 varint (7 bits per byte,
//! low group first, top bit = more, canonical, at most 3 bytes), then the payload. Type `0x00` is one padding byte with no
//! length. Every other type has a length, so any receiver can skip any frame. The connection owns [`FRAME_RELIABLE`],
//! [`FRAME_DISCONNECT`] and the reserved `0x12`/`0x13` (ignored); every other frame is handed to the caller in order.
//!
//! **Reliable frame payload:** byte 0 = channel (bits 7..6), first (bit 5), last (bit 4), bits 3..0 zero; bytes 1..3 =
//! `msg_id` u16 (per channel, +1 per fragment, wrapping); then data. A message larger than [`MAX_FRAGMENT`] is split into
//! up to [`MAX_FRAGMENTS`] fragments with consecutive ids; the first fragment's data starts with the total message length
//! as u16, checked against [`MAX_MESSAGE`] before any buffer grows. Delivery is ordered and exactly once per channel.
//! The two length bytes leave 1,022 message bytes in the first fragment, so the largest message that fits in
//! [`MAX_FRAGMENTS`] fragments is 16,382 bytes; longer ones return [`ConnError::MessageTooLarge`].
//!
//! **Files:** `header.rs` (header and frame prefix), `acks.rs` (sequence buffer and receive window), `rtt.rs` (RTT and
//! loss), `channel.rs` (send and receive side of the reliable channels), `fragment.rs` (split and reassemble limits),
//! `seal.rs` (the [`Sealer`] slot), `connection.rs` ([`Connection`] itself). Tests live in `tests/`.

mod acks;
mod channel;
mod connection;
mod fragment;
mod header;
mod rtt;
mod seal;
#[cfg(test)]
mod tests;

use std::fmt;

pub use connection::Connection;
pub use seal::{CLEAR_HEADER_LEN, ClearHeader, KEY_PHASE_MASK, Plain, Sealer, parse_clear_header};

/// Bytes of the packet header and acknowledgement block.
pub const HEADER_LEN: usize = 15;
/// Bytes reserved for the authentication tag of a sealed packet.
pub const SEAL_OVERHEAD: usize = 16;
/// Bytes left for frames in one datagram.
pub const FRAME_SPACE: usize = crate::MAX_DATAGRAM - HEADER_LEN - SEAL_OVERHEAD;
/// Reliable channels: control, events, chat, bulk.
pub const CHANNELS: usize = 4;
/// Largest data part of one reliable fragment.
pub const MAX_FRAGMENT: usize = 1024;
/// Most fragments in one message.
pub const MAX_FRAGMENTS: usize = 16;
/// Largest reliable message: 16 fragments of data less the 2-byte total length in the first one (16,382 bytes).
pub const MAX_MESSAGE: usize = MAX_FRAGMENTS * MAX_FRAGMENT - 2;
/// Most unacknowledged messages per channel.
pub const MAX_IN_FLIGHT: usize = 256;

/// Frame type: one byte of padding, no length.
pub const FRAME_PADDING: u8 = 0x00;
/// Frame type: a reliable message or fragment.
pub const FRAME_RELIABLE: u8 = 0x10;
/// Frame type: the sender closes the connection; payload is one reason byte.
pub const FRAME_DISCONNECT: u8 = 0x11;

/// Static configuration of one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnConfig {
    /// The connection id this side answers to; packets naming another id are refused.
    pub local_id: u16,
    /// The connection id the peer answers to; written into every packet.
    pub remote_id: u16,
    /// Silence after which [`Connection::is_timed_out`] turns true.
    pub idle_timeout_ms: u64,
    /// Silence after which [`Connection::wants_keepalive`] turns true.
    pub keepalive_ms: u64,
}

impl Default for ConnConfig {
    fn default() -> Self {
        Self { local_id: 0, remote_id: 0, idle_timeout_ms: 10_000, keepalive_ms: 2_000 }
    }
}

/// Why a datagram or call was refused. A refused call leaves the connection unchanged, except where a method says
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnError {
    /// The datagram is shorter than the header.
    Truncated,
    /// Reserved flag bits are set.
    BadHeader,
    /// The packet names another connection id.
    WrongConnection,
    /// The packet number was already received.
    Duplicate,
    /// The packet number is more than 1024 behind the newest received.
    TooOld,
    /// A frame is malformed: a length runs past the datagram, a varint is not canonical, or a reliable frame is invalid.
    BadFrame,
    /// The channel index is above 3.
    BadChannel,
    /// The message is above [`MAX_MESSAGE`] bytes or would need more than [`MAX_FRAGMENTS`] fragments.
    MessageTooLarge,
    /// The channel already has [`MAX_IN_FLIGHT`] unacknowledged messages.
    QueueFull,
    /// The application frames do not fit in [`FRAME_SPACE`].
    FrameTooLarge,
    /// The output buffer is smaller than [`crate::MAX_DATAGRAM`].
    BufferTooSmall,
    /// The authentication tag did not verify, or the key phase names no key this side holds. Dropped silently.
    BadTag,
    /// The sealer cannot seal: its key is used up or the packet does not fit with the tag.
    SealFailed,
}

impl fmt::Display for ConnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Truncated => "datagram shorter than the header",
            Self::BadHeader => "reserved header bits set",
            Self::WrongConnection => "packet names another connection",
            Self::Duplicate => "packet number already received",
            Self::TooOld => "packet number more than 1024 behind the newest",
            Self::BadFrame => "malformed frame",
            Self::BadChannel => "channel index above 3",
            Self::MessageTooLarge => "message too large for the reliable channels",
            Self::QueueFull => "channel has too many unacknowledged messages",
            Self::FrameTooLarge => "application frames exceed the frame space",
            Self::BufferTooSmall => "output buffer smaller than one datagram",
            Self::BadTag => "authentication tag did not verify",
            Self::SealFailed => "packet could not be sealed",
        };
        f.write_str(text)
    }
}

impl std::error::Error for ConnError {}

/// Something the caller learns about its own packets or the peer, read with [`Connection::poll_event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnEvent {
    /// The peer acknowledged this packet number.
    Delivered(u32),
    /// This packet number is written off: 3 newer packets were acknowledged, or it is older than 1 s without an
    /// acknowledgement. Its reliable fragments are resent on their own timers.
    Lost(u32),
    /// The peer sent a disconnect with this reason byte.
    Disconnected(u8),
}

/// What [`Connection::write_packet`] wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sent {
    /// The packet number that was used.
    pub number: u32,
    /// Bytes written at the start of the output buffer.
    pub len: usize,
}
