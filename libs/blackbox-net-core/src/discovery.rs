//! LAN discovery datagrams (protocol.md section 3). Both are exactly [`DATAGRAM_LEN`] bytes, so a reply is never larger
//! than the request that caused it.
//!
//! CONTRACT (to implement), multi-byte integers little-endian:
//!
//! - `pub struct Request { pub proto_major: u8, pub nonce: u64 }`: `encode(&self) -> [u8; 64]` (`NMDQ`, major, format 1,
//!   nonce, 50 zero bytes) and `decode(bytes: &[u8]) -> Result<Request, DiscoveryError>` (wrong length, magic or format,
//!   or nonzero padding are errors).
//! - `pub struct Reply { proto_major: u8, nonce: u64, proto_minor: u8, key_hash: [u8; 8], game_port: u16, players: u8,
//!   max_players: u8, flags: u8, mode: u8, data_id: [u8; 4], name: String }` (`pub` fields): `encode(&self) -> [u8; 64]`
//!   at the offsets of protocol.md section 3 (name: UTF-8, cut to at most 24 bytes on a character boundary, zero padded),
//!   `decode(bytes) -> Result<Reply, DiscoveryError>` (name must be valid UTF-8 up to the first zero; trailing name bytes
//!   and the last 7 bytes must be zero).
//! - `Reply::answers(&self, request: &Request) -> bool`: same major and nonce.
//! - Flag bits as `pub const`: `FLAG_PASSWORD = 1`, `FLAG_IN_PROGRESS = 2`, `FLAG_JOIN_IN_PROGRESS = 4`,
//!   `FLAG_MODS = 8`, `FLAG_DEDICATED = 16`.
//! - `pub enum DiscoveryError { Length, Magic, Format, Padding, Name }` with `Display` and `std::error::Error`.
//! - Tests: round trips; a long multi-byte name is cut on a boundary; every single-byte corruption of magic, format and
//!   padding is rejected; random 64-byte input never panics.

/// Size of both discovery datagrams.
pub const DATAGRAM_LEN: usize = 64;
/// Format version of the discovery datagrams.
pub const FORMAT: u8 = 1;
