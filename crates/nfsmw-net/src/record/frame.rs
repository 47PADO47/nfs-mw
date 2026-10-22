//! One frame of a replay: sync mark, kind, tick, length, payload, CRC (module docs of [`super`]).

use nfsmw_sim::Tick;

use super::crc::crc32;

/// The two bytes that start every frame.
pub const SYNC: [u8; 2] = [0xA5, 0x5A];
/// Bytes a frame adds to its payload: sync 2, kind 1, tick 4, length 4, CRC 4.
pub const FRAME_OVERHEAD: usize = 15;
/// The largest payload of one frame.
pub const MAX_FRAME_PAYLOAD: usize = 256 * 1024;
/// The kind byte's flag for a key snapshot.
const KEY_FLAG: u8 = 0x80;

/// What a frame holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameKind {
    /// A snapshot packet.
    Snapshot,
    /// A reliable event message.
    Event,
    /// The recording client's own input frame (client recordings only).
    Input,
    /// The recording client's predicted own state (client recordings only).
    Predicted,
    /// The seek index (written once, at the end).
    Index,
    /// A kind a later format added; readers skip it.
    Other(u8),
}

impl FrameKind {
    pub fn code(self) -> u8 {
        match self {
            Self::Snapshot => 1,
            Self::Event => 2,
            Self::Input => 3,
            Self::Predicted => 4,
            Self::Index => 5,
            Self::Other(code) => code & 0x0F,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Snapshot,
            2 => Self::Event,
            3 => Self::Input,
            4 => Self::Predicted,
            5 => Self::Index,
            other => Self::Other(other),
        }
    }
}

/// A frame as written: its fields and payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawFrame<'a> {
    pub kind: FrameKind,
    pub key: bool,
    pub tick: Tick,
    pub payload: &'a [u8],
}

/// Appends one frame. The caller checks the payload length first.
pub(crate) fn write_frame(out: &mut Vec<u8>, frame: &RawFrame<'_>) {
    out.extend_from_slice(&SYNC);
    let start = out.len();
    let key = match frame.key {
        true => KEY_FLAG,
        false => 0,
    };
    out.push(frame.kind.code() | key);
    out.extend_from_slice(&frame.tick.to_le_bytes());
    out.extend_from_slice(&(frame.payload.len() as u32).to_le_bytes());
    out.extend_from_slice(frame.payload);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_le_bytes());
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// The frame starting at `at` and ending at or before `end`, and the offset after it; `None` when the bytes there are
/// not a whole, intact frame.
pub(crate) fn read_frame(bytes: &[u8], at: usize, end: usize) -> Option<(RawFrame<'_>, usize)> {
    let bytes = bytes.get(..end)?;
    if bytes.get(at..at.checked_add(2)?)? != SYNC {
        return None;
    }
    let kind_byte = *bytes.get(at + 2)?;
    let tick = u32_at(bytes, at + 3)?;
    let len = usize::try_from(u32_at(bytes, at + 7)?).ok()?;
    if len > MAX_FRAME_PAYLOAD {
        return None;
    }
    let payload_start = at + 11;
    let payload_end = payload_start.checked_add(len)?;
    let payload = bytes.get(payload_start..payload_end)?;
    let stored = u32_at(bytes, payload_end)?;
    if crc32(&bytes[at + 2..payload_end]) != stored {
        return None;
    }
    let frame =
        RawFrame { kind: FrameKind::from_code(kind_byte & 0x0F), key: kind_byte & KEY_FLAG != 0, tick, payload };
    Some((frame, payload_end + 4))
}

/// The offset of the next sync mark at or after `from`, before `end`.
pub(crate) fn next_sync(bytes: &[u8], from: usize, end: usize) -> Option<usize> {
    let window = bytes.get(from..end)?;
    window.windows(2).position(|pair| pair == SYNC).map(|found| from + found)
}
