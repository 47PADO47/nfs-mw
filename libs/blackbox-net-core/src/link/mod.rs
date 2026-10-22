//! Datagram links (ADR 0009 section 3): the bottom layer, below [`crate::conn`].
//!
//! Three implementations of [`Link`]:
//!
//! - [`UdpLink`]: a non-blocking UDP socket.
//! - [`MemoryNet`] and [`MemoryLink`]: an in-process network for a listen server's own client and for tests.
//! - [`LossyLink`]: wraps any link and simulates loss, latency, jitter, duplication and reordering on a caller-driven
//!   clock, configured by [`LossConfig`].

mod memory;
mod sim;
mod udp;

#[cfg(test)]
mod tests;

pub use memory::{MemoryLink, MemoryNet};
pub use sim::{LossConfig, LossyLink};
pub use udp::UdpLink;

use std::io;
use std::net::SocketAddr;

/// A non-blocking datagram endpoint.
pub trait Link {
    /// Sends one datagram. Delivery is not guaranteed; an error means the datagram could not be handed to the network.
    fn send(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()>;

    /// Receives the next waiting datagram into `buf`, giving its source and length, or `Ok(None)` when none is waiting.
    /// A datagram longer than `buf` is truncated to `buf.len()`.
    fn recv(&mut self, buf: &mut [u8]) -> io::Result<Option<(SocketAddr, usize)>>;

    /// The address peers send to.
    fn local_addr(&self) -> SocketAddr;
}
