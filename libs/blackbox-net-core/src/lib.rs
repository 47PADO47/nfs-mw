//! Generic netcode building blocks for a server-authoritative racing game.
//! Design: `docs/decisions/0006-multiplayer-netcode.md` as amended by `docs/decisions/0009-multiplayer-netcode-revisions.md`;
//! wire layouts in `docs/specs/multiplayer/protocol.md`.
//!
//! The layers, bottom up:
//!
//! - [`bits`]: an MSB-first bit writer and reader over caller-owned byte slices, varints, gap-coded entity ids and
//!   quantisers (uniform floats, angles, smallest-three quaternions).
//! - [`link`]: the [`link::Link`] datagram trait with a UDP, an in-memory and a lossy simulator implementation.
//! - [`conn`]: a sans-IO connection: packet numbers, acknowledgement bits, RTT and loss, four reliable ordered channels
//!   with fragments, and delivery notifications for delta baselines. A [`conn::Sealer`] seals every datagram:
//!   [`conn::Plain`] on the in-memory link, the Noise sealer of `secure` on the network.
//! - [`replicate`]: entity replication over a schema the game registers: a history ring of quantised frames, delta
//!   encoding against an acknowledged baseline, an encode cache shared by clients, a priority accumulator and a
//!   spatial hash for relevancy.
//! - [`clock`]: the client's estimate of the server tick and the slew-or-step rule.
//! - [`session`]: the session and per-peer state machines (lobby, load barrier, intro, countdown, running, results),
//!   and the player party roster for local and remote seats.
//! - [`discovery`]: the LAN discovery request and reply datagrams.
//! - [`ratelimit`]: token buckets, failure locks and the pre-authentication caps.
//! - `handoff` (feature `handoff`): the bounded drop-oldest ring and the latest-value cell between the net thread and
//!   the game thread.
//! - `secure` (feature `netcode`): the cookie, the Noise handshake, sealed packets with rekeying, identity keys and
//!   trust on first use.
//!
//! No game names, no threads and no wall clock: every function that needs the time takes `now_ms` from the caller, so
//! tests are deterministic. The default build is standard library only; `handoff` adds `crossbeam-queue` and
//! `triple_buffer`, `netcode` adds `snow` and the RustCrypto crates.

pub mod bits;
pub mod clock;
pub mod conn;
pub mod discovery;
#[cfg(feature = "handoff")]
pub mod handoff;
pub mod link;
pub mod ratelimit;
pub mod replicate;
pub mod rng;
#[cfg(feature = "netcode")]
pub mod secure;
pub mod session;

/// Largest UDP payload the protocol sends; never fragmented by IP.
pub const MAX_DATAGRAM: usize = 1200;
