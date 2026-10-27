//! World animation instance and entity records. See `docs/specs/world-mesh-animation.md`.
use anyhow::{Context, Result, bail};
use blackbox_chunk::{chunks, ids::WORLD_ANIM_TREE_MARKERS};

const ENTITIES: u32 = 0x0003_7080;
const INSTANCES: u32 = 0x0003_7090;
const ENTITY_SIZE: usize = 112;
const INSTANCE_SIZE: usize = 160;

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: u32,
    pub model: u32,
    pub tree: u32,
    pub animation: u32,
    pub flags: u8,
    pub parent: Option<usize>,
    pub matrix: [f32; 16],
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub tree: u32,
    pub section: i32,
    pub flags: u32,
    pub speed: f32,
    pub master_delay: f32,
    pub loop_delay: f32,
    pub start_trigger: u32,
    pub stop_trigger: u32,
    pub track: u32,
    pub direction: i16,
    pub matrix: [f32; 16],
}

#[derive(Debug, Default)]
pub struct WorldAnimations {
    pub trees: std::collections::HashMap<u32, Vec<Entity>>,
    pub instances: Vec<Instance>,
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("fixed-size record"))
}

fn f32_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(bytes, at))
}

fn matrix(bytes: &[u8], at: usize) -> Result<[f32; 16]> {
    let mut result = [0.0; 16];
    for (i, value) in result.iter_mut().enumerate() {
        *value = f32_at(bytes, at + 4 * i);
    }
    if result.iter().any(|v| !v.is_finite()) {
        bail!("nonfinite animation matrix");
    }
    Ok(result)
}

impl WorldAnimations {
    /// Read the top-level track chunks in their authored order.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let mut result = Self::default();
        let mut pending = Vec::new();
        for chunk in chunks(bytes) {
            let chunk = chunk?;
            let data = chunk.aligned_payload(16);
            match chunk.id {
                ENTITIES => {
                    if !data.len().is_multiple_of(ENTITY_SIZE) {
                        bail!("invalid world entity length");
                    }
                    for record in data.as_chunks::<ENTITY_SIZE>().0 {
                        let parent = record[41] as i8;
                        let parent = (parent >= 0).then_some(parent as usize);
                        if parent.is_some_and(|index| index >= pending.len()) {
                            bail!("world entity parent is not before child");
                        }
                        pending.push(Entity {
                            name: u32_at(record, 4),
                            model: u32_at(record, 12),
                            tree: u32_at(record, 32),
                            animation: u32_at(record, 36),
                            flags: record[40],
                            parent,
                            matrix: matrix(record, 48)?,
                        });
                    }
                }
                WORLD_ANIM_TREE_MARKERS => {
                    let marker = data.get(..80).context("truncated world animation marker")?;
                    let name = u32_at(marker, 0);
                    if !pending.is_empty() {
                        if pending.iter().any(|entity| entity.tree != name) || result.trees.contains_key(&name) {
                            bail!("inconsistent world animation tree");
                        }
                        result.trees.insert(name, std::mem::take(&mut pending));
                    }
                }
                INSTANCES => {
                    if !data.len().is_multiple_of(INSTANCE_SIZE) {
                        bail!("invalid world instance length");
                    }
                    for record in data.as_chunks::<INSTANCE_SIZE>().0 {
                        result.instances.push(Instance {
                            tree: u32_at(record, 4),
                            section: u32_at(record, 8) as i32,
                            flags: u32_at(record, 12),
                            speed: f32_at(record, 16),
                            master_delay: f32_at(record, 20),
                            loop_delay: f32_at(record, 24),
                            start_trigger: u32_at(record, 32),
                            stop_trigger: u32_at(record, 36),
                            track: u32_at(record, 56),
                            direction: i16::from_le_bytes(record[60..62].try_into()?),
                            matrix: matrix(record, 64)?,
                        });
                    }
                }
                _ => {}
            }
        }
        if !pending.is_empty() {
            bail!("unterminated world animation tree");
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "needs installed track and global animation data"]
    fn installed_world_trees_link_to_instance_and_channels() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let data = crate::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
        let world = WorldAnimations::read(&data).unwrap();
        assert_eq!((world.trees.len(), world.instances.len()), (11, 11));
        let objects = crate::anim::AnimObjects::from_bytes(&data).unwrap();
        let clips: std::collections::HashSet<_> = objects
            .banks
            .iter()
            .flat_map(|(_, bank)| bank.iter())
            .map(|a| blackbox_hash::bstring_hash(&a.name))
            .collect();
        for instance in &world.instances {
            let tree = &world.trees[&instance.tree];
            for entity in tree.iter().filter(|e| e.animation != 0) {
                for (bit, suffix) in [(1, b"_t"), (2, b"_q"), (4, b"_s")] {
                    if entity.flags & bit != 0 {
                        assert!(clips.contains(&blackbox_hash::bstring_hash_continue(entity.animation, suffix)));
                    }
                }
            }
        }
        let global = crate::anim::AnimObjects::load(&dir, "GLOBAL/InGameB.bun").unwrap();
        assert!(global.skeletons.iter().any(|s| s.name == "ROOT" && s.bones.len() >= 2));
    }
}
