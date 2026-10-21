//! LAN discovery datagrams (protocol.md section 3). Both are exactly [`DATAGRAM_LEN`] bytes, so a reply is never larger
//! than the request that caused it. Multi-byte integers are little-endian.
//!
//! A reply is only a hint: the server's identity is proved by its key in the handshake, never taken from the reply.
//! Decoding never panics; every malformed input is a [`DiscoveryError`].

mod error;
mod name;
mod reply;
mod request;
#[cfg(test)]
mod tests;
mod wire;

pub use error::DiscoveryError;
pub use reply::Reply;
pub use request::Request;

/// Size of both discovery datagrams.
pub const DATAGRAM_LEN: usize = 64;
/// Format version of the discovery datagrams.
pub const FORMAT: u8 = 1;

/// Flag bit 0: the server needs a password.
pub const FLAG_PASSWORD: u8 = 1;
/// Flag bit 1: a race is in progress.
pub const FLAG_IN_PROGRESS: u8 = 2;
/// Flag bit 2: joining a race in progress is allowed.
pub const FLAG_JOIN_IN_PROGRESS: u8 = 4;
/// Flag bit 3: the server requires mods.
pub const FLAG_MODS: u8 = 8;
/// Flag bit 4: the server is dedicated.
pub const FLAG_DEDICATED: u8 = 16;
