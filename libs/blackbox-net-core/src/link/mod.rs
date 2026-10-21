//! Datagram links (ADR 0009 section 3): the bottom layer, below [`crate::conn`].
//!
//! CONTRACT (to implement; the skeleton only fixes the trait):
//!
//! - `udp.rs`: `pub struct UdpLink` over `std::net::UdpSocket`. `bind(addr: SocketAddr) -> io::Result<Self>` sets the
//!   socket non-blocking. `recv` maps `WouldBlock` to `Ok(None)`; on Windows `ConnectionReset` (ICMP port unreachable from
//!   an earlier send) is also `Ok(None)` and the loop continues to the next datagram.
//! - `memory.rs`: an in-process network. `MemoryNet::new() -> MemoryNet` (cheap to clone, `Send + Sync`, shared state in
//!   an `Arc<Mutex<..>>`), `MemoryNet::endpoint(&self, addr: SocketAddr) -> MemoryLink`. A send to an address with no
//!   endpoint is dropped silently (like UDP). Datagrams larger than [`crate::MAX_DATAGRAM`] are refused with
//!   `io::ErrorKind::InvalidInput`. Per-endpoint queue capacity 4096 datagrams; overflow drops the newest. Used by the
//!   listen server's own client (no socket, no handshake) and by tests.
//! - `sim.rs`: `pub struct LossyLink<L: Link>` wrapping any link, with `pub struct LossConfig { loss: f64, latency_ms: u32,
//!   jitter_ms: u32, duplicate: f64, reorder: f64, seed: u64 }` (all fields `pub`, `Default` = a perfect link).
//!   `LossyLink::new(inner, config)`, `set_now(&mut self, now_ms: u64)` advances simulated time and forwards every
//!   outgoing datagram whose delivery time has come to the inner link. On `send`, a datagram is dropped with probability
//!   `loss`, else scheduled at `now + latency + uniform(0..=jitter)`; with probability `duplicate` a second copy is
//!   scheduled with its own jitter; with probability `reorder` it gets an extra delay of `latency + jitter` so later packets
//!   overtake it. Datagrams due at the same time keep send order. Randomness from [`crate::rng::SplitMix64`] only, so a
//!   seed reproduces a run exactly. `recv` passes through to the inner link. Counters: `sent()`, `dropped()`,
//!   `duplicated()`.
//! - `tests.rs`: memory pair delivery, unknown address dropped, oversize refused; UDP on `127.0.0.1:0` (send to self, read
//!   back; empty socket gives `Ok(None)`); `LossyLink` statistics over 10,000 datagrams within ±2 % of the configured
//!   loss and duplicate rates, latency and jitter bounds respected, same seed gives the same delivery sequence, a perfect
//!   config delivers in order.

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
