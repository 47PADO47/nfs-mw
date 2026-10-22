//! The EAGL objects of a bundle: its skeletons (`EAGLSkeletons`) and its animation banks (`EAGLAnimations`), read with
//! `blackbox-anim` from the bundle's chunk tree. The formats are in `docs/formats/animation.md`; this module only finds
//! the chunks and reads each one.

use anyhow::{Context, Result};
use blackbox_anim::{Bank, Object, Skeleton};
use blackbox_chunk::find_all;
use blackbox_chunk::ids::{EAGL_ANIMATIONS, EAGL_SKELETONS};
use game_install::GameDir;

use crate::read_unwrapped;

/// The skeleton payloads' object starts on a 16-byte boundary inside the chunk payload.
const SKELETON_ALIGN: usize = 16;

/// Every skeleton and animation bank of one bundle, in chunk order (containers searched depth first).
#[derive(Debug, Clone, Default)]
pub struct AnimObjects {
    /// The skeletons the bundle holds.
    pub skeletons: Vec<Skeleton>,
    /// The animation banks, each with the object its compound and channel records live in.
    pub banks: Vec<(Object, Bank)>,
}

impl AnimObjects {
    /// Reads every skeleton and animation bank chunk of an unwrapped bundle. Any chunk that does not read is an error
    /// that names its offset; bytes with no such chunks give empty lists.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let skeletons = find_all(bytes, EAGL_SKELETONS)
            .into_iter()
            .map(|chunk| {
                let at = chunk.offset;
                let object = Object::read(chunk.aligned_payload(SKELETON_ALIGN))
                    .with_context(|| format!("skeleton object at {at:#x}"))?;
                Skeleton::read(&object).with_context(|| format!("skeleton at {at:#x}"))
            })
            .collect::<Result<Vec<_>>>()?;
        let banks = find_all(bytes, EAGL_ANIMATIONS)
            .into_iter()
            .map(|chunk| {
                let at = chunk.offset;
                let object = Object::read(chunk.payload).with_context(|| format!("bank object at {at:#x}"))?;
                let bank = Bank::read(&object).with_context(|| format!("bank at {at:#x}"))?;
                Ok((object, bank))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { skeletons, banks })
    }

    /// Reads the bundle at `relative_path` in the install (its compression wrapper removed) and every object in it.
    pub fn load(dir: &GameDir, relative_path: &str) -> Result<Self> {
        let data = read_unwrapped(dir, relative_path)?;
        Self::from_bytes(&data).with_context(|| format!("reading the EAGL objects of {relative_path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::AnimObjects;
    use blackbox_chunk::{CONTAINER_BIT, ids::EAGL_SKELETONS};

    /// One chunk: id, size, then the payload.
    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = id.to_le_bytes().to_vec();
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
        out
    }

    #[test]
    fn bytes_without_eagl_chunks_give_no_objects() {
        let unrelated = chunk(0x1234_5678, &[1, 2, 3, 4]);
        let nested = chunk(CONTAINER_BIT | 0x0BAD_0001, &[unrelated.clone(), unrelated].concat());
        let objects = AnimObjects::from_bytes(&nested).expect("no EAGL chunk is not an error");
        assert!(objects.skeletons.is_empty() && objects.banks.is_empty());
        assert!(AnimObjects::from_bytes(&[]).expect("empty bytes").skeletons.is_empty());
    }

    #[test]
    fn a_broken_skeleton_inside_a_container_names_its_offset() {
        let broken = chunk(EAGL_SKELETONS, &[0; 8]);
        let container = chunk(CONTAINER_BIT | 0x0BAD_0002, &broken);
        let err = AnimObjects::from_bytes(&container).expect_err("a payload without an EAGL object does not read");
        let message = format!("{err:#}");
        assert!(message.contains("skeleton object at 0x8"), "{message}");
    }
}
