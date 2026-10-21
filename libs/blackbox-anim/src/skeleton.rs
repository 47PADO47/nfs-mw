//! EAGL skeletons: the bones of a rig, read from a skeleton object, with their bind pose and matrices.
//!
//! The layout is in `docs/specs/eagl-animation-skeleton.md` section 3. The object holds one `__Skeleton:::<rig>`
//! symbol, whose value is the offset of a 16-byte header. Before the header are one 16-byte index record per bone
//! (`__Bone:::<rig>.<bone>` symbols point at them), and after it come the 112-byte bone records.

use glam::{Mat4, Quat, Vec3};

use crate::bytes;
use crate::object::Object;
use crate::pose::{Pose, Sqt};
use crate::{Error, Result};

const SKELETON_PREFIX: &str = "__Skeleton:::";
const BONE_PREFIX: &str = "__Bone:::";
/// The size of an index record: the bone's index in its first 4 bytes, then 12 unused bytes.
const INDEX_RECORD_LEN: usize = 16;
/// The size of the header that follows the index records.
const HEADER_LEN: usize = 16;
const BONE_LEN: usize = 112;
const WHAT_HEADER: &str = "skeleton header";
const WHAT_INDEX: &str = "bone index record";
const WHAT_BONE: &str = "bone record";

/// A rig: its bones in index order, which is parent-first.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// The rig name, the text after `:::` in the `__Skeleton` symbol.
    pub name: String,
    /// The rig checksum from the header.
    pub checksum: u16,
    /// The bones. Bone `i` has its parent at an index smaller than `i`, or no parent.
    pub bones: Vec<Bone>,
}

/// One joint of a skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The bone name, the text after `<rig>.` in its `__Bone` symbol.
    pub name: String,
    /// The index of the parent bone, or `None` for a root.
    pub parent: Option<usize>,
    /// The bind transform, in the parent's frame.
    pub bind: Sqt,
    /// The inverse of the bind world matrix. It maps the bind pose back to the rest of the rig.
    pub inverse_bind: Mat4,
}

/// The header fields that the reader needs.
struct Header {
    /// The offset of the header in `.data`.
    at: usize,
    checksum: u16,
    count: usize,
}

impl Skeleton {
    /// Reads the skeleton of an object. The object must hold exactly one `__Skeleton` symbol, and every
    /// `__Bone` symbol must belong to that rig and point at an index record.
    pub fn read(object: &Object) -> Result<Self> {
        let (name, at) = rig_symbol(object)?;
        let header = read_header(&object.data, at)?;
        let names = bone_names(object, &name, &header)?;
        let bones = names
            .into_iter()
            .enumerate()
            .map(|(index, bone_name)| read_bone(&object.data, &header, index, bone_name))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { name, checksum: header.checksum, bones })
    }

    /// The bind pose: every bone at its bind transform.
    pub fn bind_pose(&self) -> Pose {
        Pose { bones: self.bones.iter().map(|bone| bone.bind).collect() }
    }

    /// The index of the bone called `name`, if there is one.
    pub fn bone_index(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|bone| bone.name == name)
    }

    /// The world matrix of every bone for `pose`. A root's world matrix is its local matrix. A child's is its
    /// parent's world matrix times its own local matrix. Panics if `pose` has a different number of bones.
    pub fn world(&self, pose: &Pose) -> Vec<Mat4> {
        assert_eq!(pose.bones.len(), self.bones.len(), "the pose and the skeleton have different bone counts");
        let mut world: Vec<Mat4> = Vec::with_capacity(self.bones.len());
        for (bone, sqt) in self.bones.iter().zip(&pose.bones) {
            let local = sqt.matrix();
            let global = match bone.parent {
                Some(parent) => world[parent] * local,
                None => local,
            };
            world.push(global);
        }
        world
    }

    /// The skinning matrix of every bone for `pose`: its world matrix times its inverse bind matrix. For the bind
    /// pose these are the identity. Panics if `pose` has a different number of bones.
    pub fn skinning(&self, pose: &Pose) -> Vec<Mat4> {
        self.world(pose).into_iter().zip(&self.bones).map(|(world, bone)| world * bone.inverse_bind).collect()
    }
}

/// The rig name and the header offset from the single `__Skeleton` symbol.
fn rig_symbol(object: &Object) -> Result<(String, usize)> {
    let mut symbols = object.symbols_with_prefix(SKELETON_PREFIX);
    let Some(symbol) = symbols.next() else {
        return Err(Error::Invalid("the object has no __Skeleton symbol".to_string()));
    };
    if symbols.next().is_some() {
        return Err(Error::Invalid("the object has more than one __Skeleton symbol".to_string()));
    }
    let name = symbol.name[SKELETON_PREFIX.len()..].to_string();
    Ok((name, symbol.value as usize))
}

/// Reads the header at `at` and checks that the bone records after it fit in `.data`. The count is checked
/// before anything is allocated for it.
fn read_header(data: &[u8], at: usize) -> Result<Header> {
    let checksum = bytes::u16_at(data, at, WHAT_HEADER)?;
    let raw = bytes::u32_at(data, at + 8, WHAT_HEADER)? as i32;
    let count = usize::try_from(raw).map_err(|_| Error::Invalid(format!("the skeleton has {raw} bones")))?;
    let bones_end = BONE_LEN.checked_mul(count).and_then(|len| len.checked_add(at + HEADER_LEN));
    if bones_end.is_none_or(|end| end > data.len()) {
        return Err(Error::Truncated { what: WHAT_BONE, offset: data.len() });
    }
    Ok(Header { at, checksum, count })
}

/// The name of every bone, in index order. Each `__Bone` symbol points at an index record whose first word is the
/// bone's index.
fn bone_names(object: &Object, rig: &str, header: &Header) -> Result<Vec<String>> {
    let prefix = format!("{BONE_PREFIX}{rig}.");
    let mut names: Vec<Option<String>> = vec![None; header.count];
    for symbol in object.symbols_with_prefix(BONE_PREFIX) {
        let Some(name) = symbol.name.strip_prefix(&prefix) else {
            return Err(Error::Invalid(format!("bone symbol {} is not of rig {rig}", symbol.name)));
        };
        let at = symbol.value as usize;
        if at >= header.at || !at.is_multiple_of(INDEX_RECORD_LEN) {
            return Err(Error::Invalid(format!("bone symbol {} does not point at an index record", symbol.name)));
        }
        let index = bytes::u32_at(&object.data, at, WHAT_INDEX)? as usize;
        let Some(slot) = names.get_mut(index) else {
            return Err(Error::Invalid(format!(
                "bone {name} has index {index}, but the rig has {} bones",
                header.count
            )));
        };
        if slot.is_some() {
            return Err(Error::Invalid(format!("two bone symbols have index {index}")));
        }
        *slot = Some(name.to_string());
    }
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| name.ok_or_else(|| Error::Invalid(format!("no bone symbol has index {index}"))))
        .collect()
}

/// Reads the bone record of bone `index`. The parent must come before the bone, or be `-1` for a root.
fn read_bone(data: &[u8], header: &Header, index: usize, name: String) -> Result<Bone> {
    let at = header.at + HEADER_LEN + BONE_LEN * index;
    let parent = parent_of(bytes::u32_at(data, at + 0x0C, WHAT_BONE)? as i32, index)?;
    let bind =
        Sqt { scale: vec3_at(data, at)?, rotation: quat_at(data, at + 0x10)?, translation: vec3_at(data, at + 0x20)? };
    let mut inverse = [0f32; 16];
    for (k, value) in inverse.iter_mut().enumerate() {
        *value = bytes::f32_at(data, at + 0x30 + 4 * k, WHAT_BONE)?;
    }
    Ok(Bone { name, parent, bind, inverse_bind: Mat4::from_cols_array(&inverse) })
}

/// `-1` is a root. Any other parent must be an earlier bone.
fn parent_of(raw: i32, index: usize) -> Result<Option<usize>> {
    if raw == -1 {
        return Ok(None);
    }
    let parent = usize::try_from(raw)
        .ok()
        .filter(|&parent| parent < index)
        .ok_or_else(|| Error::Invalid(format!("bone {index} has parent {raw}, which is not an earlier bone")))?;
    Ok(Some(parent))
}

fn f32_at(data: &[u8], at: usize) -> Result<f32> {
    bytes::f32_at(data, at, WHAT_BONE)
}

/// Three floats, x then y then z.
fn vec3_at(data: &[u8], at: usize) -> Result<Vec3> {
    Ok(Vec3::new(f32_at(data, at)?, f32_at(data, at + 4)?, f32_at(data, at + 8)?))
}

/// A quaternion stored as x, y, z, w.
fn quat_at(data: &[u8], at: usize) -> Result<Quat> {
    Ok(Quat::from_xyzw(f32_at(data, at)?, f32_at(data, at + 4)?, f32_at(data, at + 8)?, f32_at(data, at + 12)?))
}
