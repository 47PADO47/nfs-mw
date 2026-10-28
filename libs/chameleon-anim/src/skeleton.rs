//! A character's skeleton (resource type 0xB2). Spec: `docs/formats/chameleon-driver-animation.md`.

use crate::bytes::Block;
use crate::error::{Error, Result};

/// Offset of the bone count (u16), of the offset of the bone table and of the offset of the table of name hashes (the
/// word at 8 holds the offsets of the bones of the figure that the game looks up by name).
const COUNT_AT: usize = 2;
const BONES_AT: usize = 4;
const HASHES_AT: usize = 0x0C;
/// Bytes of one bone record: position, rotation, then the links.
const BONE_SIZE: usize = 0x30;
const ROTATION_AT: usize = 0x10;
const PARENT_AT: usize = 0x20;

/// The hash the skeleton stores for the bone called `name`: the bit-inverted CRC-32 of the name.
pub fn bone_hash(name: &str) -> u32 {
    let mut crc = u32::MAX;
    for byte in name.bytes() {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}

/// One bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    pub hash: u32,
    /// The index of the parent; `None` for the root.
    pub parent: Option<usize>,
    /// Where the bone sits at rest, metres in the figure's space (x left, y up, z forward), not relative to the parent.
    pub position: [f32; 3],
    /// The rest rotation, a quaternion (x, y, z, w).
    pub rotation: [f32; 4],
}

/// The bones of a figure, in file order; a parent comes before its children.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    /// Parse the main block of a skeleton resource.
    pub fn parse(main: &[u8]) -> Result<Self> {
        let block = Block::new(main, "skeleton");
        let count = usize::from(block.u16(COUNT_AT)?);
        let (bones_at, hashes_at) = (block.u32(BONES_AT)? as usize, block.u32(HASHES_AT)? as usize);
        let mut bones = Vec::with_capacity(count);
        for n in 0..count {
            let record = bones_at.saturating_add(n * BONE_SIZE);
            let parent = i32::from_le_bytes(block.u32(record + PARENT_AT)?.to_le_bytes());
            let parent = usize::try_from(parent).ok();
            if parent.is_some_and(|p| p >= n) {
                return Err(Error::Invalid { what: "skeleton", detail: format!("bone {n} comes before its parent") });
            }
            bones.push(Bone {
                hash: block.u32(hashes_at.saturating_add(4 * n))?,
                parent,
                position: block.f32s::<3>(record)?,
                rotation: block.f32s::<4>(record + ROTATION_AT)?,
            });
        }
        Ok(Self { bones })
    }

    /// The index of the bone called `name`.
    pub fn find(&self, name: &str) -> Option<usize> {
        let hash = bone_hash(name);
        self.bones.iter().position(|bone| bone.hash == hash)
    }

    /// Whether `bone` is `ancestor` or below it.
    pub fn is_below(&self, bone: usize, ancestor: usize) -> bool {
        let mut at = Some(bone);
        while let Some(current) = at {
            if current == ancestor {
                return true;
            }
            at = self.bones.get(current).and_then(|b| b.parent);
        }
        false
    }
}
