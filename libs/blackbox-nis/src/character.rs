//! One character record of a scene (`AnimSceneEntityData`, `0x00037040`): the actor's instance name, its clip and
//! skeleton hashes, its models (LOD names), its play settings and its local placement.
//! Layout: `docs/specs/nis-scenes.md`, section 4.

use glam::Mat4;

use crate::bytes::{f32_at, u32_at};
use crate::{Error, Result};

/// Bytes of one record in the chunk: 8 bytes of `0x11` alignment padding, then the 144-byte record.
pub const RECORD_LEN: usize = 152;
/// Bytes of alignment padding at the start of a record.
const PAD_LEN: usize = 8;
const WHAT: &str = "character record";

/// One character of a scene, read from its record. Hashes are the engine's `bStringHash` values; the record stores no
/// strings, so none can be resolved here.
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// Entity kind: 1 for a basic character, 2 for a prop, 3 for a world animation node.
    pub type_id: u32,
    /// Name hash of the actor inside the scene.
    pub instance_name_hash: u32,
    /// Name hash of the parent entity; 0 for the scene root.
    pub parent_name_hash: u32,
    /// Bit set that picks the play mode.
    pub play_flags: u32,
    /// Local placement in scene space. Column-major: the translation is the fourth column.
    pub local_matrix: Mat4,
    /// Name hashes of the skinned model's LOD levels. Zero where a level is absent.
    pub lod_name_hashes: [u32; 4],
    /// Bank animation for the root translation; 0 for none.
    pub translation_anim_hash: u32,
    /// Bank animation for the rotations; 0 for none.
    pub quaternion_anim_hash: u32,
    /// Bank animation for the scale; 0 for none.
    pub scale_anim_hash: u32,
    /// The actor's clip: a bank animation of the same bundle.
    pub skeletal_anim_hash: u32,
    /// The skeleton that the clip drives.
    pub skeleton_name_hash: u32,
    /// Multiplier on the clip's time.
    pub play_speed: f32,
    /// Seconds before the clip starts.
    pub play_delay: f32,
    /// World object type hash, used only for props.
    pub world_object_type_hash: u32,
    /// Loop range start in frames, used only with the loop flag.
    pub loop_start: u32,
    /// Loop range end in frames, used only with the loop flag.
    pub loop_end: u32,
}

impl Character {
    /// Reads one record from the payload of its chunk. The payload must be exactly 152 bytes, and its first 8 bytes
    /// must be `0x11` alignment padding. The padding is skipped by position, not by value, after the check.
    pub fn read(record: &[u8]) -> Result<Self> {
        if record.len() < RECORD_LEN {
            return Err(Error::Truncated { what: WHAT, offset: record.len() });
        }
        if record.len() > RECORD_LEN {
            return Err(Error::Invalid(format!("character record is {} bytes, not {RECORD_LEN}", record.len())));
        }
        if record[..PAD_LEN].iter().any(|&byte| byte != blackbox_chunk::PAD_BYTE) {
            return Err(Error::Invalid("character record does not start with 8 bytes of 0x11 padding".into()));
        }
        let data = &record[PAD_LEN..];

        let mut matrix = [0.0f32; 16];
        for (index, value) in matrix.iter_mut().enumerate() {
            *value = f32_at(data, 0x10 + 4 * index, WHAT)?;
        }
        let mut lod_name_hashes = [0u32; 4];
        for (index, hash) in lod_name_hashes.iter_mut().enumerate() {
            *hash = u32_at(data, 0x50 + 4 * index, WHAT)?;
        }
        Ok(Self {
            type_id: u32_at(data, 0x00, WHAT)?,
            instance_name_hash: u32_at(data, 0x04, WHAT)?,
            parent_name_hash: u32_at(data, 0x08, WHAT)?,
            play_flags: u32_at(data, 0x0C, WHAT)?,
            local_matrix: Mat4::from_cols_array(&matrix),
            lod_name_hashes,
            translation_anim_hash: u32_at(data, 0x60, WHAT)?,
            quaternion_anim_hash: u32_at(data, 0x64, WHAT)?,
            scale_anim_hash: u32_at(data, 0x68, WHAT)?,
            skeletal_anim_hash: u32_at(data, 0x6C, WHAT)?,
            skeleton_name_hash: u32_at(data, 0x70, WHAT)?,
            play_speed: f32_at(data, 0x74, WHAT)?,
            play_delay: f32_at(data, 0x78, WHAT)?,
            world_object_type_hash: u32_at(data, 0x7C, WHAT)?,
            loop_start: u32_at(data, 0x80, WHAT)?,
            loop_end: u32_at(data, 0x84, WHAT)?,
        })
    }
}
