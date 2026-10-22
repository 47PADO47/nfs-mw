//! `PING` and `PONG` payloads (protocol.md section 6.4).

use blackbox_net_core::bits::{BitReader, BitWriter};

use super::FrameError;
use crate::clock::{Ping, Pong};

/// Bytes of a PING payload: seq 8, client time 32.
pub const PING_LEN: usize = 5;
/// Bytes of a PONG payload: seq 8, client time 32, two ticks of 32 + 8 bits.
pub const PONG_LEN: usize = 15;

/// Largest tick a PONG can carry, in 1/256 ticks.
const MAX_FIXED: f64 = (u64::MAX >> 24) as f64;

/// The PING payload.
pub fn encode_ping(ping: &Ping) -> [u8; PING_LEN] {
    let mut out = [0u8; PING_LEN];
    out[0] = ping.seq;
    out[1..].copy_from_slice(&ping.client_time_ms.to_be_bytes());
    out
}

/// A PING payload of exactly [`PING_LEN`] bytes.
pub fn decode_ping(bytes: &[u8]) -> Result<Ping, FrameError> {
    let [seq, a, b, c, d] = *bytes else {
        return Err(FrameError::BadLength);
    };
    Ok(Ping { seq, client_time_ms: u32::from_be_bytes([a, b, c, d]) })
}

/// The PONG payload. A tick that is negative, not finite or past `u32::MAX` is [`FrameError::BadValue`].
pub fn encode_pong(pong: &Pong) -> Result<[u8; PONG_LEN], FrameError> {
    let recv = fixed(pong.server_recv_tick)?;
    let send = fixed(pong.server_send_tick)?;
    let mut out = [0u8; PONG_LEN];
    let mut w = BitWriter::new(&mut out);
    let fields = [(u64::from(pong.seq), 8), (u64::from(pong.client_time_ms), 32), (recv, 40), (send, 40)];
    for (value, bits) in fields {
        w.write_bits(value, bits).map_err(|_| FrameError::BadLength)?;
    }
    Ok(out)
}

/// A PONG payload of exactly [`PONG_LEN`] bytes.
pub fn decode_pong(bytes: &[u8]) -> Result<Pong, FrameError> {
    if bytes.len() != PONG_LEN {
        return Err(FrameError::BadLength);
    }
    let mut r = BitReader::new(bytes);
    let mut take = |bits: u32| r.read_bits(bits).map_err(|_| FrameError::BadLength);
    let seq = take(8)? as u8;
    let client_time_ms = take(32)? as u32;
    let recv = take(40)? as f64 / 256.0;
    let send = take(40)? as f64 / 256.0;
    Ok(Pong { seq, client_time_ms, server_recv_tick: recv, server_send_tick: send })
}

/// A tick in 1/256 ticks: 32 bits of tick and 8 of fraction.
fn fixed(tick: f64) -> Result<u64, FrameError> {
    let scaled = (tick * 256.0).round();
    if !scaled.is_finite() || scaled < 0.0 || scaled > MAX_FIXED {
        return Err(FrameError::BadValue);
    }
    Ok(scaled as u64)
}
