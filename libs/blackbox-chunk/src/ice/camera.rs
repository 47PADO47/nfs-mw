use super::bytes::{f32_at, i16_at, u32_at};
use super::error::{IceError, IceResult};
use crate::ids::ICE_NIS_CAMERAS;
use crate::{Chunk, chunks};

/// Bytes of a camera record before its elements. Offsets 0..8 are rewritten by the engine when the
/// record is linked into its list, so their on-disk value is not meaningful.
pub const CAMERA_RECORD_HEADER_LEN: usize = 0x28;
/// Element count, a signed 16-bit value inside the record header.
const RECORD_COUNT_OFFSET: usize = 0x14;
/// Stride of one camera element.
pub const CAMERA_ELEMENT_LEN: usize = 0x84;
/// Element byte 0: the evaluator skips the element when this byte is zero.
const ELEMENT_FLAG_OFFSET: usize = 0;
/// Float time key at element offset 0x10. The engine searches it backwards for the last key <= t.
const ELEMENT_TIME_OFFSET: usize = 0x10;

/// One ICE camera pack (`0x8003B200..=0x8003B203`).
#[derive(Debug, Clone, PartialEq)]
pub struct CameraPack<'a> {
    /// 0 = NIS, 1 = FMV, 2 = MKR, 3 = replay. The engine's own names, not checked against the decomp.
    pub kind: u32,
    pub sets: Vec<CameraSet<'a>>,
}

/// One sub-chunk of a camera pack.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraSet<'a> {
    /// Dedup key: a second set with the same key is merged into the first one by the engine.
    pub key: u32,
    pub records: Vec<CameraRecord<'a>>,
}

/// One camera record: a 0x28-byte header followed by `element_count` elements.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraRecord<'a> {
    pub header: &'a [u8],
    pub elements: Vec<&'a [u8]>,
}

/// Parses an ICE camera pack chunk (`0x8003B200..=0x8003B203`).
pub fn parse_camera_pack<'a>(chunk: &Chunk<'a>) -> IceResult<CameraPack<'a>> {
    let kind = camera_kind(chunk.id)?;
    let sets = chunks(chunk.payload).map(|sub| parse_set(sub?.payload)).collect::<IceResult<Vec<_>>>()?;
    Ok(CameraPack { kind, sets })
}

fn camera_kind(id: u32) -> IceResult<u32> {
    let kind = id.wrapping_sub(ICE_NIS_CAMERAS);
    if kind > 3 {
        return Err(IceError::WrongId(id));
    }
    Ok(kind)
}

fn parse_set(payload: &[u8]) -> IceResult<CameraSet<'_>> {
    let key = u32_at(payload, 0).ok_or(IceError::Truncated(0))?;
    let count = u32_at(payload, 4).ok_or(IceError::Truncated(4))? as i32;
    let mut records = Vec::new();
    let mut at = 8;
    for _ in 0..count.max(0) {
        let (record, next) = parse_record(payload, at)?;
        records.push(record);
        at = next;
    }
    Ok(CameraSet { key, records })
}

/// Returns the record and the offset of the record that follows it.
fn parse_record(payload: &[u8], at: usize) -> IceResult<(CameraRecord<'_>, usize)> {
    let header_end = at + CAMERA_RECORD_HEADER_LEN;
    let header = payload.get(at..header_end).ok_or(IceError::Truncated(at))?;
    let count = i16_at(header, RECORD_COUNT_OFFSET).ok_or(IceError::Truncated(at))?;
    let count_usize = usize::try_from(count).map_err(|_| IceError::NegativeCount(at, count))?;
    let end = header_end + count_usize * CAMERA_ELEMENT_LEN;
    let body = payload.get(header_end..end).ok_or(IceError::Truncated(header_end))?;
    let elements = body.as_chunks::<CAMERA_ELEMENT_LEN>().0.iter().map(|e| e.as_slice()).collect();
    Ok((CameraRecord { header, elements }, end))
}

/// True when the engine evaluates this element (byte 0 is non-zero). `None` if the slice is empty.
pub fn element_enabled(element: &[u8]) -> Option<bool> {
    let flag = *element.get(ELEMENT_FLAG_OFFSET)?;
    Some(flag != 0)
}

/// The float time key at element offset 0x10.
pub fn element_time(element: &[u8]) -> Option<f32> {
    f32_at(element, ELEMENT_TIME_OFFSET)
}
