//! The `SNAPSHOT` payload (protocol.md section 6.3 as built; layout in the module docs of [`super`]).

use blackbox_net_core::bits::{BitReader, BitWriter};
use nfsmw_sim::Tick;

use super::{FrameError, GLOBAL_BLOCK_BITS, GlobalBlock};
use crate::schema::{GameSchema, OWN_BLOCK_BITS, OwnBlock};

/// Bytes of the fixed header: part 4, parts 4, flags 8, `ack_input` 16.
pub const SNAPSHOT_HEADER_BYTES: usize = 4;
/// Most parts of one snapshot.
pub const MAX_PARTS: u8 = 8;

const FLAG_OWN: u8 = 1 << 0;
const FLAG_GLOBAL: u8 = 1 << 1;
const OWN_BYTES: usize = OWN_BLOCK_BITS / 8;
const GLOBAL_BYTES: usize = GLOBAL_BLOCK_BITS / 8;
/// Bytes of the replicate body's tick.
const TICK_BYTES: usize = 4;

/// The fixed header of one snapshot part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotHeader {
    /// This part, `0..parts`.
    pub part: u8,
    /// Parts of the snapshot, `1..=8`.
    pub parts: u8,
    /// Low 16 bits of the last input tick the server applied for this client.
    pub ack_input: u16,
}

/// One decoded snapshot part. `body` is the replicate snapshot body, still encoded: it is decoded against the
/// client's views once every part has arrived.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotFrame<'a> {
    pub header: SnapshotHeader,
    /// The snapshot's server tick (the first 32 bits of the body).
    pub tick: Tick,
    pub own: Option<OwnBlock>,
    pub global: Option<GlobalBlock>,
    pub body: &'a [u8],
}

/// Reads one snapshot part. The blocks are decoded; the body is only checked to start with a tick.
pub fn read_snapshot<'a>(schema: &GameSchema, bytes: &'a [u8]) -> Result<SnapshotFrame<'a>, FrameError> {
    let [index, flags, ack_high, ack_low, ..] = *bytes else {
        return Err(FrameError::BadLength);
    };
    let (part, parts) = (index >> 4, index & 0x0F);
    if parts == 0 || parts > MAX_PARTS || part >= parts {
        return Err(FrameError::BadPart);
    }
    if flags & !(FLAG_OWN | FLAG_GLOBAL) != 0 {
        return Err(FrameError::Reserved);
    }
    let has_own = flags & FLAG_OWN != 0;
    let has_global = flags & FLAG_GLOBAL != 0;
    if part != 0 && (has_own || has_global) {
        return Err(FrameError::BadPart);
    }
    let header = SnapshotHeader { part, parts, ack_input: u16::from_be_bytes([ack_high, ack_low]) };
    let mut at = SNAPSHOT_HEADER_BYTES;
    let mut own = None;
    if has_own {
        let block = bytes.get(at..at + OWN_BYTES).ok_or(FrameError::BadLength)?;
        own = Some(schema.read_own(&mut BitReader::new(block)).map_err(|_| FrameError::BadBlock)?);
        at += OWN_BYTES;
    }
    let mut global = None;
    if has_global {
        let block = bytes.get(at..at + GLOBAL_BYTES).ok_or(FrameError::BadLength)?;
        global = Some(GlobalBlock::read(&mut BitReader::new(block))?);
        at += GLOBAL_BYTES;
    }
    let body = bytes.get(at..).ok_or(FrameError::BadLength)?;
    let tick_bytes: [u8; TICK_BYTES] =
        body.get(..TICK_BYTES).ok_or(FrameError::BadLength)?.try_into().map_err(|_| FrameError::BadLength)?;
    Ok(SnapshotFrame { header, tick: Tick::from_be_bytes(tick_bytes), own, global, body })
}

/// Writes one snapshot part into `out` (cleared first): the header, the blocks given (part 0 only), then `body`, a
/// replicate snapshot body the caller encoded. A block on another part, or a bad part number, is
/// [`FrameError::BadPart`].
pub fn write_snapshot(
    schema: &GameSchema,
    header: SnapshotHeader,
    own: Option<&OwnBlock>,
    global: Option<&GlobalBlock>,
    body: &[u8],
    out: &mut Vec<u8>,
) -> Result<(), FrameError> {
    out.clear();
    if header.parts == 0 || header.parts > MAX_PARTS || header.part >= header.parts {
        return Err(FrameError::BadPart);
    }
    if header.part != 0 && (own.is_some() || global.is_some()) {
        return Err(FrameError::BadPart);
    }
    if body.len() < TICK_BYTES {
        return Err(FrameError::BadLength);
    }
    let flags = (u8::from(own.is_some()) * FLAG_OWN) | (u8::from(global.is_some()) * FLAG_GLOBAL);
    out.push((header.part << 4) | header.parts);
    out.push(flags);
    out.extend_from_slice(&header.ack_input.to_be_bytes());
    if let Some(own) = own {
        let mut block = [0u8; OWN_BYTES];
        schema.write_own(own, &mut BitWriter::new(&mut block)).map_err(|_| FrameError::BadBlock)?;
        out.extend_from_slice(&block);
    }
    if let Some(global) = global {
        let mut block = [0u8; GLOBAL_BYTES];
        global.write(&mut BitWriter::new(&mut block))?;
        out.extend_from_slice(&block);
    }
    out.extend_from_slice(body);
    Ok(())
}
