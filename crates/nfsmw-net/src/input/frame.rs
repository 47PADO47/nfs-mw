//! The `INPUT` frame payload (protocol.md section 6.2). The connection layer writes the frame type and length around it.
//!
//! Layout, most significant bit first: `first_tick` 16 (low 16 bits), `count` 4 (1..=8), `flags` 4 (bit 0 the session
//! uses a clutch, bit 1 some input carries a gear, bits 2 and 3 zero), then per input either `same` = 1 (equal to the
//! input before; never on the first) or `same` = 0, throttle 7, brake 7, steer 9, handbrake 4, nos, shift up, shift
//! down, then the gear (a present bit and 4 bits when present) if flag bit 1, then the clutch 4 if flag bit 0. Zero
//! padding to the byte.
//!
//! Canonical form: `same` is used whenever an input equals the one before, the gear flag is set exactly when some input
//! carries a gear, and no byte follows the padding. A decoder refuses anything else.

use blackbox_net_core::bits::{BitError, BitReader, BitWriter};

use super::InputError;
use super::code::{InputCode, STEER_MAX};

/// The frame type of an input frame.
pub const FRAME_INPUT: u8 = 0x01;
/// Most inputs in one frame.
pub const MAX_FRAME_TICKS: usize = 8;
/// Most payload bytes: a 3-byte header and 8 inputs of at most 40 bits.
pub const MAX_INPUT_PAYLOAD: usize = 3 + MAX_FRAME_TICKS * 40 / 8;

const FLAG_CLUTCH: u64 = 1 << 0;
const FLAG_GEAR: u64 = 1 << 1;

/// Up to [`MAX_FRAME_TICKS`] consecutive ticks of one player's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputFrame {
    first_tick: u16,
    uses_clutch: bool,
    count: usize,
    inputs: [InputCode; MAX_FRAME_TICKS],
}

impl InputFrame {
    /// An empty frame starting at the tick whose low 16 bits are `first_tick`. `uses_clutch` is the session's setting:
    /// without it the clutch is not sent and reads as zero.
    pub fn new(first_tick: u16, uses_clutch: bool) -> Self {
        Self { first_tick, uses_clutch, count: 0, inputs: [InputCode::NEUTRAL; MAX_FRAME_TICKS] }
    }

    /// Appends the next tick's input. [`InputError::Full`] after 8; [`InputError::BadValue`] for an invalid code.
    pub fn push(&mut self, code: InputCode) -> Result<(), InputError> {
        if self.count >= MAX_FRAME_TICKS {
            return Err(InputError::Full);
        }
        if !code.is_valid() {
            return Err(InputError::BadValue);
        }
        let clutch = match self.uses_clutch {
            true => code.clutch,
            false => 0,
        };
        let slot = self.inputs.get_mut(self.count).ok_or(InputError::Full)?;
        *slot = InputCode { clutch, ..code };
        self.count += 1;
        Ok(())
    }

    /// The low 16 bits of the first input's tick.
    pub fn first_tick(&self) -> u16 {
        self.first_tick
    }

    /// Whether the clutch travels in this frame.
    pub fn uses_clutch(&self) -> bool {
        self.uses_clutch
    }

    /// The inputs, oldest first.
    pub fn inputs(&self) -> &[InputCode] {
        self.inputs.get(..self.count).unwrap_or(&[])
    }

    /// How many inputs the frame holds.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether the frame holds no input.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Writes the payload into `out` and returns its length in bytes (at most [`MAX_INPUT_PAYLOAD`]).
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, InputError> {
        if self.is_empty() {
            return Err(InputError::Empty);
        }
        let gear = self.inputs().iter().any(|code| code.gear_select.is_some());
        let mut flags = 0;
        if self.uses_clutch {
            flags |= FLAG_CLUTCH;
        }
        if gear {
            flags |= FLAG_GEAR;
        }
        let mut w = BitWriter::new(out);
        w.write_bits(u64::from(self.first_tick), 16).map_err(write_error)?;
        w.write_bits(self.count as u64, 4).map_err(write_error)?;
        w.write_bits(flags, 4).map_err(write_error)?;
        let mut previous: Option<&InputCode> = None;
        for code in self.inputs() {
            let same = previous == Some(code);
            previous = Some(code);
            w.write_bool(same).map_err(write_error)?;
            if same {
                continue;
            }
            write_code(&mut w, code, gear, self.uses_clutch).map_err(write_error)?;
        }
        w.align().map_err(write_error)?;
        Ok(w.bytes_written())
    }

    /// Reads a payload written by [`InputFrame::encode`]. Every malformed or non-canonical payload is an error.
    pub fn decode(payload: &[u8]) -> Result<Self, InputError> {
        if payload.len() > MAX_INPUT_PAYLOAD {
            return Err(InputError::TrailingBytes);
        }
        let mut r = BitReader::new(payload);
        let first_tick = r.read_bits(16).map_err(read_error)? as u16;
        let count = r.read_bits(4).map_err(read_error)? as usize;
        if count == 0 || count > MAX_FRAME_TICKS {
            return Err(InputError::BadCount);
        }
        let flags = r.read_bits(4).map_err(read_error)?;
        if flags & !(FLAG_CLUTCH | FLAG_GEAR) != 0 {
            return Err(InputError::Reserved);
        }
        let uses_clutch = flags & FLAG_CLUTCH != 0;
        let gear = flags & FLAG_GEAR != 0;
        let mut frame = Self::new(first_tick, uses_clutch);
        let mut previous: Option<InputCode> = None;
        for _ in 0..count {
            let same = r.read_bool().map_err(read_error)?;
            let code = match (same, previous) {
                (true, None) => return Err(InputError::SameOnFirst),
                (true, Some(before)) => before,
                (false, _) => read_code(&mut r, gear, uses_clutch)?,
            };
            if !same && previous == Some(code) {
                return Err(InputError::NotCanonical);
            }
            frame.push(code)?;
            previous = Some(code);
        }
        if gear && !frame.inputs().iter().any(|code| code.gear_select.is_some()) {
            return Err(InputError::NotCanonical);
        }
        r.align().map_err(read_error)?;
        if r.remaining_bits() != 0 {
            return Err(InputError::TrailingBytes);
        }
        Ok(frame)
    }
}

fn write_code(w: &mut BitWriter<'_>, code: &InputCode, gear: bool, clutch: bool) -> Result<(), BitError> {
    w.write_bits(u64::from(code.throttle), 7)?;
    w.write_bits(u64::from(code.brake), 7)?;
    w.write_bits(u64::from(code.steer), 9)?;
    w.write_bits(u64::from(code.handbrake), 4)?;
    w.write_bool(code.nos)?;
    w.write_bool(code.shift_up)?;
    w.write_bool(code.shift_down)?;
    if gear {
        w.write_bool(code.gear_select.is_some())?;
        if let Some(id) = code.gear_select {
            w.write_bits(u64::from(id), 4)?;
        }
    }
    if clutch {
        w.write_bits(u64::from(code.clutch), 4)?;
    }
    Ok(())
}

fn read_code(r: &mut BitReader<'_>, gear: bool, clutch: bool) -> Result<InputCode, InputError> {
    let mut bits = |n: u32| r.read_bits(n).map_err(read_error);
    let throttle = bits(7)? as u8;
    let brake = bits(7)? as u8;
    let steer = bits(9)? as u16;
    let handbrake = bits(4)? as u8;
    let nos = bits(1)? != 0;
    let shift_up = bits(1)? != 0;
    let shift_down = bits(1)? != 0;
    let mut gear_select = None;
    if gear && bits(1)? != 0 {
        gear_select = Some(bits(4)? as u8);
    }
    let mut clutch_code = 0;
    if clutch {
        clutch_code = bits(4)? as u8;
    }
    if steer > STEER_MAX || (shift_up && shift_down) {
        return Err(InputError::BadValue);
    }
    Ok(InputCode { throttle, brake, steer, handbrake, nos, shift_up, shift_down, gear_select, clutch: clutch_code })
}

fn write_error(_: BitError) -> InputError {
    InputError::BufferTooSmall
}

fn read_error(error: BitError) -> InputError {
    match error {
        BitError::BadPadding => InputError::BadPadding,
        _ => InputError::Truncated,
    }
}
