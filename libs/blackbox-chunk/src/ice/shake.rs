use super::bytes::{i16_at, u32_at};
use super::error::{IceError, IceResult};
use crate::Chunk;
use crate::ids::ICE_SHAKE_TRACKS;

/// Bytes of a shake record before its elements (0..8 rewritten at link time, see camera).
pub const SHAKE_RECORD_HEADER_LEN: usize = 0x20;
/// Element count, a signed 16-bit value inside the record header.
const RECORD_COUNT_OFFSET: usize = 0x0C;
/// Stride of one shake element. The contents are not decoded yet.
pub const SHAKE_ELEMENT_LEN: usize = 0x18;

/// The `ICEShakeTracks` chunk (`0x0003B211`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShakeTracks<'a> {
    pub records: Vec<ShakeRecord<'a>>,
}

/// One shake record: a 0x20-byte header followed by `element_count` raw 24-byte elements.
#[derive(Debug, Clone, PartialEq)]
pub struct ShakeRecord<'a> {
    pub header: &'a [u8],
    pub elements: Vec<&'a [u8]>,
}

/// Parses an `ICEShakeTracks` chunk. The payload is a record count, then the records.
pub fn parse_shake_tracks<'a>(chunk: &Chunk<'a>) -> IceResult<ShakeTracks<'a>> {
    if chunk.id != ICE_SHAKE_TRACKS {
        return Err(IceError::WrongId(chunk.id));
    }
    let payload = chunk.payload;
    let count = u32_at(payload, 0).ok_or(IceError::Truncated(0))? as i32;
    let mut records = Vec::new();
    let mut at = 4;
    for _ in 0..count.max(0) {
        let (record, next) = parse_record(payload, at)?;
        records.push(record);
        at = next;
    }
    Ok(ShakeTracks { records })
}

fn parse_record(payload: &[u8], at: usize) -> IceResult<(ShakeRecord<'_>, usize)> {
    let header_end = at + SHAKE_RECORD_HEADER_LEN;
    let header = payload.get(at..header_end).ok_or(IceError::Truncated(at))?;
    let count = i16_at(header, RECORD_COUNT_OFFSET).ok_or(IceError::Truncated(at))?;
    let count_usize = usize::try_from(count).map_err(|_| IceError::NegativeCount(at, count))?;
    let end = header_end + count_usize * SHAKE_ELEMENT_LEN;
    let body = payload.get(header_end..end).ok_or(IceError::Truncated(header_end))?;
    let elements = body.as_chunks::<SHAKE_ELEMENT_LEN>().0.iter().map(|e| e.as_slice()).collect();
    Ok((ShakeRecord { header, elements }, end))
}
