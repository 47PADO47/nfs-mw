//! The per-tick input of a player car on the wire (protocol.md section 6.2).
//!
//! - [`InputCode`]: one tick of [`blackbox_vehicle::InputState`] quantised to the wire steps (throttle and brake 7 bits,
//!   steer 9 bits around 255, handbrake and clutch 4 bits, three buttons, an optional 4-bit gear). Quantising cleans up:
//!   a non-finite axis reads as zero, ranges are clamped, both shift edges at once become neither, a gear above 15 is
//!   dropped.
//! - [`InputFrame`]: the payload of an `INPUT` frame ([`FRAME_INPUT`]): the low 16 bits of the first tick, 1 to 8
//!   inputs, and a `same` bit for a tick equal to the one before. Decoding refuses every malformed or non-canonical
//!   frame (`security-gameplay.md` section 1.1), so whatever decodes re-encodes to the same bytes.
//! - [`InputHistory`] (client): the inputs it sent, by tick. [`InputHistory::frame`] builds each frame from the newest
//!   ticks the server has not acknowledged, so every input is repeated in up to 8 datagrams and a lost one costs nothing.
//!   Prediction replays from the same history.
//! - [`InputWindow`] (server, one per client): takes frames in, drops ticks already simulated and ticks too far ahead
//!   (`security-gameplay.md` section 4.3), and hands out the input of each tick, holding the last one for 100 ms when
//!   one is missing and then falling back to a gentle brake.
//! - [`expand_tick`]: a 16-bit tick back to the full tick nearest a reference.

mod code;
mod frame;
mod history;
mod window;

#[cfg(test)]
mod tests;

pub use code::{CLUTCH_MAX, GEAR_MAX, HANDBRAKE_MAX, InputCode, PEDAL_MAX, STEER_CENTRE, STEER_MAX};
pub use frame::{FRAME_INPUT, InputFrame, MAX_FRAME_TICKS, MAX_INPUT_PAYLOAD};
pub use history::{HISTORY_TICKS, InputHistory};
pub use window::{Accepted, HOLD_TICKS, InputSource, InputWindow, MAX_AHEAD_TICKS};

use nfsmw_sim::Tick;

/// Why an input frame could not be built, encoded or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    #[error("the frame holds no input")]
    Empty,
    #[error("the frame already holds 8 inputs")]
    Full,
    #[error("the input count is 0 or above 8")]
    BadCount,
    #[error("a value is outside its field")]
    BadValue,
    #[error("a reserved bit is set")]
    Reserved,
    #[error("the first input of a frame says `same`")]
    SameOnFirst,
    #[error("the frame is valid but not in canonical form")]
    NotCanonical,
    #[error("the payload ended inside a field")]
    Truncated,
    #[error("the padding bits are not zero")]
    BadPadding,
    #[error("bytes follow the padding")]
    TrailingBytes,
    #[error("the output buffer is too small")]
    BufferTooSmall,
}

/// The full tick of `low`, the low 16 bits of a tick, taken as the tick nearest `reference` (valid within ±32,768
/// ticks, about 9 minutes at 60 Hz).
pub fn expand_tick(low: u16, reference: Tick) -> Tick {
    let delta = low.wrapping_sub(reference as u16) as i16;
    reference.wrapping_add_signed(i32::from(delta))
}
