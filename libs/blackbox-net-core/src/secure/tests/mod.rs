//! Tests of the secure channel: packet codecs and seeded garbage for every pre-authentication parser, the handshake
//! (direct and over the seeded lossy link), sealing with replay and tamper rejection and rekeying, connect tokens, the
//! room password, identity keys and trust on first use.

mod garbage;
mod handshake;
mod harness;
mod lossy;
mod packets;
mod password;
mod sealing;
mod tokens;
mod trust;
