//! A car's skeleton: the bones its body is skinned to. Spec: `docs/formats/chameleon-hinged-parts.md`.
//!
//! The body of a 2012 car is one skinned model. Its skeleton (resource type 0xB2) has one bone per articulated piece:
//! lamps, doors, mirrors, bumpers, the bonnet, the steering wheel. A bone's name is not stored; its hash is, and the hash
//! is the bit-inverted CRC-32 of the name, so a bone is found by name (`BonnetHinge`, `SteeringWheel`).

use chameleon_bundle::{Bundle, ResourceKind};

/// Offset of the bone count (u16) in the main block.
const COUNT_AT: usize = 2;
/// Offset of the offset of the bone table, and of the offset of the table of name hashes.
const BONES_AT: usize = 4;
const HASHES_AT: usize = 8;
/// Bytes of one bone record: position and rotation, then the links.
const BONE_SIZE: usize = 0x30;
/// Offset of the parent index (i32, -1 for the root) inside a record.
const PARENT_AT: usize = 0x20;

/// One bone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    /// The bit-inverted CRC-32 of the bone's name.
    pub hash: u32,
    /// The index of the parent bone; `None` for the root.
    pub parent: Option<usize>,
    /// Where the bone sits at rest, metres in the engine's space (x left, y up, z forward).
    pub position: [f32; 3],
}

/// The bones of a car, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

/// The hash the skeleton stores for the bone called `name`.
pub fn bone_hash(name: &str) -> u32 {
    let mut crc = u32::MAX;
    for byte in name.bytes() {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    // The standard CRC-32 is `!crc`; the skeleton keeps the register as it is.
    crc
}

fn word(block: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?))
}

impl Skeleton {
    /// Read the skeleton of a car bundle; `None` when it has none or it is malformed.
    pub fn read(bundle: &Bundle) -> Option<Self> {
        let kind = ResourceKind::Skeleton.id();
        let index = bundle.entries().iter().position(|entry| entry.kind == kind)?;
        Self::parse(&bundle.resource(index).ok()?.blocks[0])
    }

    /// Parse the main block of a skeleton resource.
    pub fn parse(main: &[u8]) -> Option<Self> {
        let count = usize::from(u16::from_le_bytes(main.get(COUNT_AT..COUNT_AT + 2)?.try_into().ok()?));
        let (bones_at, hashes_at) = (word(main, BONES_AT)? as usize, word(main, HASHES_AT)? as usize);
        let mut bones = Vec::with_capacity(count);
        for n in 0..count {
            let record = bones_at.checked_add(n * BONE_SIZE)?;
            let float = |at: usize| word(main, record + at).map(f32::from_bits);
            let parent = word(main, record + PARENT_AT)? as i32;
            let parent = usize::try_from(parent).ok().filter(|&p| p < count && p != n);
            bones.push(Bone {
                hash: word(main, hashes_at.checked_add(4 * n)?)?,
                parent,
                position: [float(0)?, float(4)?, float(8)?],
            });
        }
        Some(Self { bones })
    }

    /// The index of the bone called `name`.
    pub fn find(&self, name: &str) -> Option<usize> {
        let hash = bone_hash(name);
        self.bones.iter().position(|bone| bone.hash == hash)
    }

    /// The nearest bone of `marked` on the way from `bone` up to the root, `bone` itself included.
    pub fn nearest(&self, bone: usize, marked: &[usize]) -> Option<usize> {
        let mut at = Some(bone);
        for _ in 0..=self.bones.len() {
            let current = at?;
            if marked.contains(&current) {
                return Some(current);
            }
            at = self.bones.get(current)?.parent;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bone_names_hash_to_what_the_skeleton_stores() {
        // Checked on the 911: these are the hashes of its root, bonnet hinge and steering wheel bones.
        assert_eq!(bone_hash("Root"), 0x4939_A99A);
        assert_eq!(bone_hash("BonnetHinge"), 0x59CD_5076);
        assert_eq!(bone_hash("SteeringWheel"), 0xCC70_82F3);
    }

    fn block(bones: &[(u32, i32)]) -> Vec<u8> {
        let mut out = vec![0u8; 0x10];
        out[2..4].copy_from_slice(&(bones.len() as u16).to_le_bytes());
        out[4..8].copy_from_slice(&0x10u32.to_le_bytes());
        let hashes_at = 0x10 + bones.len() * BONE_SIZE;
        out[8..12].copy_from_slice(&(hashes_at as u32).to_le_bytes());
        for &(_, parent) in bones {
            let mut record = [0u8; BONE_SIZE];
            record[PARENT_AT..PARENT_AT + 4].copy_from_slice(&parent.to_le_bytes());
            out.extend_from_slice(&record);
        }
        for &(hash, _) in bones {
            out.extend_from_slice(&hash.to_le_bytes());
        }
        out
    }

    #[test]
    fn the_nearest_marked_ancestor_wins() {
        let skeleton = Skeleton::parse(&block(&[(1, -1), (2, 0), (3, 1), (4, 2), (5, 0)])).unwrap();
        assert_eq!(skeleton.bones[3].parent, Some(2));
        assert_eq!(skeleton.bones[0].parent, None);
        assert_eq!(skeleton.nearest(3, &[1, 2]), Some(2));
        assert_eq!(skeleton.nearest(3, &[1]), Some(1));
        assert_eq!(skeleton.nearest(4, &[1]), None);
    }

    #[test]
    fn a_truncated_skeleton_is_refused() {
        let mut data = block(&[(1, -1), (2, 0)]);
        data.truncate(0x20);
        assert!(Skeleton::parse(&data).is_none());
    }
}
