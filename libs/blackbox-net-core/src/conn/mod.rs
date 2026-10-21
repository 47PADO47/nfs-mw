//! A sans-IO connection (protocol.md section 5, ADR 0009 sections 3 and 5): it turns bytes into bytes and never touches a
//! socket or a clock. The caller moves datagrams through a [`crate::link::Link`] and passes `now_ms`.
//!
//! CONTRACT (to implement; the skeleton only fixes the constants and the frame rule):
//!
//! **Packet layout** (all multi-byte integers little-endian):
//!
//! | Offset | Size | Field |
//! |---|---|---|
//! | 0 | 1 | flags: bit 7 = 0 (data packet), bits 3..0 = format 0, others 0 |
//! | 1 | 2 | connection id of the receiver |
//! | 3 | 4 | packet number (starts at 0, +1 per packet) |
//! | 7 | 4 | `ack`: largest packet number received from the peer (0 with `ack_bits` 0 before any) |
//! | 11 | 4 | `ack_bits`: bit `i` set means packet `ack - 1 - i` was received |
//! | 15 | n | frames, at most [`FRAME_SPACE`] bytes, so a 16-byte seal tag still fits in [`crate::MAX_DATAGRAM`] later |
//!
//! **Frames** (ADR 0009 section 4): `type` u8, then the payload length as a byte-aligned LEB128 varint (7 bits per byte,
//! low group first, top bit = more, canonical, at most 3 bytes), then the payload. Type `0x00` is one padding byte with no
//! length. Every other type has a length, so any receiver can skip any frame. The connection owns [`FRAME_RELIABLE`],
//! [`FRAME_DISCONNECT`] and the reserved `0x12`/`0x13`; every other frame is handed to the caller in order.
//!
//! **Reliable frame payload:** byte 0 = channel (bits 7..6), first (bit 5), last (bit 4), bits 3..0 zero; bytes 1..3 =
//! `msg_id` u16 (per channel, +1 per fragment, wrapping); then data. A message larger than [`MAX_FRAGMENT`] is split into
//! up to [`MAX_FRAGMENTS`] fragments with consecutive ids; the first fragment's data starts with the total message length
//! as u16, checked against [`MAX_MESSAGE`] before any buffer grows. Delivery is ordered and exactly once per channel.
//!
//! **Files:**
//!
//! - `acks.rs`: `pub struct SequenceBuffer<T>` (a ring of `Option<T>` keyed by `u32` with a fixed capacity:
//!   `with_capacity(n)`, `insert(seq, value)` (overwrites the slot), `get`, `get_mut`, `remove`, `contains`), and the
//!   received-window tracker that builds `ack`/`ack_bits` and rejects duplicates and numbers more than 1024 older than the
//!   newest ([`ConnError::Duplicate`], [`ConnError::TooOld`]). Wrap-aware comparison `seq_newer(a, b)`.
//! - `rtt.rs`: smoothed RTT and variance (RFC 6298 gains 1/8 and 1/4; first sample sets both; initial 100 ms), loss as an
//!   exponential average (gain 1/16) of lost (1) and delivered (0) outcomes.
//! - `channel.rs`: send side (a queue of at most [`MAX_IN_FLIGHT`] unacknowledged messages per channel, fragment state,
//!   last send time; a fragment is due when never sent or when `now - last_sent >= max(20, 1.5 * srtt)` ms) and receive side
//!   (out-of-order fragments buffered within the 256-message window, reassembly, in-order delivery). Buffers are created
//!   once and reused; delivered messages are copied out.
//! - `fragment.rs`: split and reassemble, with all limits checked before allocating.
//! - `header.rs`: write and parse the 15-byte header and the frame type/length prefix.
//! - `mod.rs` (this file): `pub struct ConnConfig { local_id: u16, remote_id: u16, idle_timeout_ms: u64 (10_000),
//!   keepalive_ms: u64 (2_000) }` (`pub` fields, `Default`), and `pub struct Connection` with:
//!   - `new(config) -> Self`;
//!   - `send_message(&mut self, channel: u8, msg: &[u8]) -> Result<(), ConnError>` (`BadChannel`, `MessageTooLarge`,
//!     `QueueFull`);
//!   - `write_packet(&mut self, now_ms: u64, app_frames: &[(u8, &[u8])], out: &mut [u8]) -> Result<Sent, ConnError>`:
//!     `out.len() >= MAX_DATAGRAM`; header and acks, then the caller's frames (`FrameTooLarge` if they do not fit in
//!     [`FRAME_SPACE`]), then a pending disconnect, then due reliable fragments oldest first while they fit. Always writes
//!     a packet (acks only if nothing else). Records what each packet carried in a `SequenceBuffer`.
//!     `pub struct Sent { pub number: u32, pub len: usize }`;
//!   - `read_packet<'a, F: FnMut(u8, &'a [u8])>(&mut self, now_ms: u64, datagram: &'a [u8], on_frame: F) ->
//!     Result<u32, ConnError>`: checks length (`Truncated`), flags (`BadHeader`), connection id (`WrongConnection`), the
//!     window, then every frame (`BadFrame` stops parsing; frames already handed out stay handed out); applies the acks:
//!     each newly acknowledged packet gives `ConnEvent::Delivered(number)`, marks its reliable fragments delivered and
//!     feeds one RTT sample; an unacknowledged packet at least 3 below the newest acknowledged one, or older than 1 s,
//!     gives `ConnEvent::Lost(number)` once;
//!   - `poll_event(&mut self) -> Option<ConnEvent>`; `pub enum ConnEvent { Delivered(u32), Lost(u32),
//!     Disconnected(u8) }`;
//!   - `recv_message(&mut self, out: &mut Vec<u8>) -> Option<u8>`: clears `out`, copies the next complete message
//!     (channels 0 to 3 in order) and returns its channel;
//!   - `close(&mut self, reason: u8)` (sends one DISCONNECT frame, payload = the reason byte, in the next packet);
//!   - `is_timed_out(now_ms) -> bool`, `wants_keepalive(now_ms) -> bool`, `has_due_reliable(now_ms) -> bool`,
//!     `rtt_ms() -> f64`, `loss() -> f64`, `bytes_sent() -> u64`, `bytes_received() -> u64`.
//!   - `pub enum ConnError { Truncated, BadHeader, WrongConnection, Duplicate, TooOld, BadFrame, BadChannel,
//!     MessageTooLarge, QueueFull, FrameTooLarge, BufferTooSmall }` with `Display` and `std::error::Error`.
//! - `tests/`: two connections wired by a test harness that moves datagrams and drops, delays, duplicates and reorders
//!   them with [`crate::rng::SplitMix64`] (no `Link` needed). At 0, 2, 5 and 10 % loss, 2,000 messages of 1 byte to 16 KiB
//!   on all four channels arrive in order exactly once, and the send queues drain to empty. Delivered/Lost events cover
//!   every sent packet exactly once. A seeded fuzz loop (100,000 random and mutated datagrams) never panics. With no
//!   reliable traffic, `write_packet` and `read_packet` do not allocate after the first 100 packets (checked in A6's
//!   allocation test; keep the hot path free of `Vec` growth).

/// Bytes of the packet header and acknowledgement block.
pub const HEADER_LEN: usize = 15;
/// Bytes reserved for the authentication tag of the future sealed packet.
pub const SEAL_OVERHEAD: usize = 16;
/// Bytes left for frames in one datagram.
pub const FRAME_SPACE: usize = crate::MAX_DATAGRAM - HEADER_LEN - SEAL_OVERHEAD;
/// Reliable channels: control, events, chat, bulk.
pub const CHANNELS: usize = 4;
/// Largest data part of one reliable fragment.
pub const MAX_FRAGMENT: usize = 1024;
/// Most fragments in one message.
pub const MAX_FRAGMENTS: usize = 16;
/// Largest reliable message.
pub const MAX_MESSAGE: usize = 16 * 1024;
/// Most unacknowledged messages per channel.
pub const MAX_IN_FLIGHT: usize = 256;

/// Frame type: one byte of padding, no length.
pub const FRAME_PADDING: u8 = 0x00;
/// Frame type: a reliable message or fragment.
pub const FRAME_RELIABLE: u8 = 0x10;
/// Frame type: the sender closes the connection; payload is one reason byte.
pub const FRAME_DISCONNECT: u8 = 0x11;
