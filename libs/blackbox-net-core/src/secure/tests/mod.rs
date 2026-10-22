//! Tests of the secure channel: packet codecs, the handshake (direct and over the seeded lossy link), sealing with
//! replay and tamper rejection and rekeying, identity keys and trust on first use.

mod handshake;
mod harness;
mod lossy;
mod packets;
mod sealing;
mod trust;
