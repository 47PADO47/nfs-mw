//! The client-side recording's own frames (MP-72): the predicted own-car state, beside the received snapshots and the
//! client's encoded input frames. A recording of both peers at one tick shows where prediction and the server parted.

use nfsmw_sim::Tick;

/// Bytes of an encoded [`PredictedState`]: the tick it was predicted from and 13 little-endian `f32`s.
pub const PREDICTED_STATE_LEN: usize = 4 + 13 * 4;

/// The client's predicted own car at a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PredictedState {
    /// The last server tick the prediction replayed from.
    pub base_tick: Tick,
    pub position: [f32; 3],
    /// Unit quaternion `[x, y, z, w]`.
    pub orientation: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
}

impl PredictedState {
    /// Appends the fixed [`PREDICTED_STATE_LEN`]-byte form.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.base_tick.to_le_bytes());
        let floats =
            self.position.iter().chain(&self.orientation).chain(&self.linear_velocity).chain(&self.angular_velocity);
        for value in floats {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }

    /// Reads the fixed form; `None` for another length.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PREDICTED_STATE_LEN {
            return None;
        }
        let word = |index: usize| {
            let at = 4 + index * 4;
            f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        Some(Self {
            base_tick: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            position: [word(0), word(1), word(2)],
            orientation: [word(3), word(4), word(5), word(6)],
            linear_velocity: [word(7), word(8), word(9)],
            angular_velocity: [word(10), word(11), word(12)],
        })
    }
}
