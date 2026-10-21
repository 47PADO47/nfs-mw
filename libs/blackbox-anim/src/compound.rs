//! A compound channel (type 15): the record each bank entry points to. It carries the animation's frame count and
//! frame rate, and the child channels that hold its data. Layout: `docs/specs/eagl-animation-channels.md` section 3
//! and `docs/specs/eagl-animation-bank.md` section 4.

use crate::{Error, Object, Result, TypeId};

const TYPE: usize = 0x00;
const CHECKSUM: usize = 0x02;
const ATTRIBUTES: usize = 0x04;
const CHILD_COUNT: usize = 0x08;
const FRAME_COUNT: usize = 0x0A;
const CHILDREN: usize = 0x0C;
const POINTER_LEN: usize = 4;

/// The attribute block's entry count is a u32 at its start; the entries follow it.
const ATTRIBUTE_COUNT_LEN: usize = 4;
/// One attribute entry: id (u16), size (u16), then a 4-byte value field.
const ATTRIBUTE_LEN: usize = 8;
const ATTRIBUTE_ID: usize = 0;
const ATTRIBUTE_SIZE: usize = 2;
const ATTRIBUTE_VALUE: usize = 4;
/// The attribute id of the frame rate (`ID_FPS` in the decomp).
const FPS_ID: u16 = 1;
/// The frame rate used when the attribute block cannot give one (bank spec section 10.1).
///
/// The reader falls back to this when the attribute pointer is null or unreadable, the block has no FPS entry, or the
/// entry is not a one-byte value of 0. Every compound in the install holds 30, so the default matches the install.
/// A block that gives a usable value overrides it.
pub const DEFAULT_FPS: f32 = 30.0;

/// A compound channel and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct Compound {
    /// The target checksum at offset 2: the skeleton the animation drives.
    pub checksum: u16,
    /// The animation length in frames, from offset 0x0A.
    pub frames: u16,
    /// Frames per second, from the attribute block. Defaults to [`DEFAULT_FPS`] (see that constant).
    pub fps: f32,
    /// The child channels, in record order.
    pub children: Vec<ChildRef>,
}

/// A child channel of a compound: its type id and where its record starts in `.data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildRef {
    /// The child's type id (a delta codec in the install; see [`TypeId::is_supported_leaf`]).
    pub type_id: u16,
    /// The offset in `.data` of the child record.
    pub offset: u32,
}

impl Compound {
    /// Reads the compound record at `offset` in `.data`.
    ///
    /// The record must have type 15. Each child pointer must be a relocated pointer to a record whose type id is
    /// one of the known ids; a null or out-of-range child pointer is an error.
    pub fn read(object: &Object, offset: u32) -> Result<Self> {
        let at = offset as usize;
        let type_id = object.u16_at(at + TYPE)?;
        if type_id != TypeId::Compound as u16 {
            return Err(Error::Invalid(format!(
                "channel at {offset:#x} has type {type_id}, not a compound ({})",
                TypeId::Compound as u16
            )));
        }
        let checksum = object.u16_at(at + CHECKSUM)?;
        let fps = object
            .ptr_at(at + ATTRIBUTES)
            .ok()
            .flatten()
            .and_then(|block| read_fps(object, block as usize))
            .unwrap_or(DEFAULT_FPS);
        let child_count = object.u16_at(at + CHILD_COUNT)?;
        let frames = object.u16_at(at + FRAME_COUNT)?;
        let mut children = Vec::new();
        for index in 0..usize::from(child_count) {
            let slot = at + CHILDREN + index * POINTER_LEN;
            let target = object.ptr_at(slot)?.ok_or_else(|| {
                Error::Invalid(format!("compound at {offset:#x} has a null pointer for child {index}"))
            })?;
            children.push(read_child(object, target)?);
        }
        Ok(Self { checksum, frames, fps, children })
    }

    /// The length of the animation in seconds: the frame count divided by the frame rate.
    ///
    /// The whole frame count counts, so a 41-frame animation at 30 fps lasts 41/30 s. This is the rule the bank spec
    /// gives (`docs/specs/eagl-animation-bank.md` section 10.3).
    pub fn duration_secs(&self) -> f32 {
        f32::from(self.frames) / self.fps
    }
}

/// The child record at `target`: its type id must be a known one.
fn read_child(object: &Object, target: u32) -> Result<ChildRef> {
    let type_id = object.u16_at(target as usize)?;
    if TypeId::from_u16(type_id).is_none() {
        return Err(Error::Invalid(format!("child at {target:#x} has unknown type id {type_id}")));
    }
    Ok(ChildRef { type_id, offset: target })
}

/// The frame rate in the attribute block at `block`, or `None` when the block cannot give one.
///
/// The FPS entry has size 1 and its value is the first byte of the value field (0x1E, which is 30, in every
/// install block; bank spec section 10.1). Another reading of that byte, as the top byte of the 32-bit field, is
/// listed there as unconfirmed. Any read failure, a missing entry, a size other than 1 or a zero value gives `None`.
fn read_fps(object: &Object, block: usize) -> Option<f32> {
    let count = object.u32_at(block).ok()?;
    for index in 0..count {
        let entry = block + ATTRIBUTE_COUNT_LEN + usize::try_from(index).ok()? * ATTRIBUTE_LEN;
        let id = object.u16_at(entry + ATTRIBUTE_ID).ok()?;
        if id != FPS_ID {
            continue;
        }
        let size = object.u16_at(entry + ATTRIBUTE_SIZE).ok()?;
        if size != 1 {
            return None;
        }
        let value = object.u8_at(entry + ATTRIBUTE_VALUE).ok()?;
        return (value != 0).then(|| f32::from(value));
    }
    None
}
