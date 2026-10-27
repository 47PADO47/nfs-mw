//! The `SpeedSoundSet` object of `SOUND/SPEEDSOUNDS.BNDL`: the layers of the sound of speed (wind, the world rushing by, the
//! flybys of other cars). Spec: `docs/formats/chameleon-sound-graph.md`, "The sound of speed".
//!
//! The install holds exactly one. It is a `Genesys.Gen.SpeedSoundSet` of 396 bytes that holds, inline and in this order, two
//! `WindLayer`s (68 bytes), four `WorldLayer`s (36), two `FlybyLayer`s (28), an id and eleven floats. A layer names its
//! samples by handle: a wind layer four slots in its body, the others an array behind an offset and a count. Field names are
//! hashed, so the values are kept in the order of the type record, without names.

use std::collections::HashMap;

use chameleon_bundle::{Bundle, ResourceKind};

use crate::bytes::{f32_at, u16_at, u32_at};
use crate::graphs::type_names;

const TYPE_FLAG: u32 = 0x8000_0000;
const CLASS: &str = "Genesys.Gen.SpeedSoundSet";
/// Where the layers sit in the object.
const WIND_AT: [usize; 2] = [12, 80];
const WORLD_AT: [usize; 4] = [148, 184, 220, 256];
const FLYBY_AT: [usize; 2] = [292, 320];
/// The floats of the set itself, after its id at 348.
const SET_FLOATS_AT: usize = 352;
const SET_FLOATS: usize = 11;

/// A wind layer: four sample slots and five floats (the type record lists them in this order).
#[derive(Debug, Clone, PartialEq)]
pub struct WindLayer {
    pub samples: [u64; 4],
    pub values: [f32; 5],
}

/// A layer of the world: the samples of an array and three floats.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldLayer {
    pub samples: Vec<u64>,
    pub values: [f32; 3],
}

/// A flyby layer: the samples of an array and one float.
#[derive(Debug, Clone, PartialEq)]
pub struct FlybyLayer {
    pub samples: Vec<u64>,
    pub value: f32,
}

/// The set of speed sounds.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedSoundSet {
    pub id: u64,
    pub wind: Vec<WindLayer>,
    pub world: Vec<WorldLayer>,
    pub flyby: Vec<FlybyLayer>,
    /// The eleven floats of the set itself.
    pub values: Vec<f32>,
}

impl SpeedSoundSet {
    /// The set of `bundle`, `None` when the bundle has no `SpeedSoundSet` object.
    pub fn read(bundle: &Bundle) -> Option<Self> {
        let type_kind = ResourceKind::GenesysType.id();
        let names = type_names(bundle, type_kind);
        let wanted: Vec<u64> = names.iter().filter(|(_, name)| name.as_str() == CLASS).map(|(id, _)| *id).collect();
        for (index, entry) in bundle.entries().iter().enumerate() {
            if entry.kind != ResourceKind::GenesysObject.id() {
                continue;
            }
            let Ok(resource) = bundle.resource(index) else { continue };
            let is_set = resource.imports.iter().any(|i| i.offset == TYPE_FLAG && wanted.contains(&i.id));
            if !is_set {
                continue;
            }
            let handles: HashMap<usize, u64> = resource
                .imports
                .iter()
                .filter(|i| !names.contains_key(&i.id) && i.offset & TYPE_FLAG != 0)
                .map(|i| ((i.offset & !TYPE_FLAG) as usize, i.id))
                .collect();
            return Self::parse(entry.id, &resource.blocks[0], &handles);
        }
        None
    }

    /// Parse the main block of the object `id`; `handles` maps the offset of a handle slot to the resource it names.
    pub fn parse(id: u64, block: &[u8], handles: &HashMap<usize, u64>) -> Option<Self> {
        let handle = |at: usize| handles.get(&at).copied().unwrap_or(0);
        let floats =
            |at: usize, count: usize| -> Option<Vec<f32>> { (0..count).map(|n| f32_at(block, at + 4 * n)).collect() };
        let array = |at: usize, count_at: usize| -> Option<Vec<u64>> {
            let begin = u32_at(block, at)? as usize;
            let count = usize::from(u16_at(block, count_at)?);
            Some((0..count).map(|n| handle(begin + 8 * n)).filter(|&sample| sample != 0).collect())
        };
        let mut wind = Vec::new();
        for at in WIND_AT {
            let values = floats(at + 48, 5)?;
            let samples = [handle(at + 12), handle(at + 20), handle(at + 28), handle(at + 36)];
            wind.push(WindLayer { samples, values: [values[0], values[1], values[2], values[3], values[4]] });
        }
        let mut world = Vec::new();
        for at in WORLD_AT {
            let values = floats(at + 20, 3)?;
            world.push(WorldLayer { samples: array(at + 16, at + 32)?, values: [values[0], values[1], values[2]] });
        }
        let mut flyby = Vec::new();
        for at in FLYBY_AT {
            flyby.push(FlybyLayer { samples: array(at + 16, at + 24)?, value: f32_at(block, at + 20)? });
        }
        Some(Self { id, wind, world, flyby, values: floats(SET_FLOATS_AT, SET_FLOATS)? })
    }

    /// The sample every wind and world slot names, when they all name the same one: in the shipped data it is a 100 ms
    /// stretch of digital silence, a placeholder, so the layers play nothing.
    pub fn placeholder(&self) -> Option<u64> {
        let first = self.wind.first()?.samples[0];
        let wind = self.wind.iter().flat_map(|layer| layer.samples);
        let world = self.world.iter().flat_map(|layer| layer.samples.iter().copied());
        (first != 0 && wind.chain(world).all(|sample| sample == first)).then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(block: &mut [u8], at: usize, value: f32) {
        block[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// A block laid out like the shipped object, with `silent` in every wind and world slot.
    fn sample_object(silent: u64) -> (Vec<u8>, HashMap<usize, u64>) {
        let mut block = vec![0u8; 864];
        let mut handles = HashMap::new();
        for (n, at) in WIND_AT.into_iter().enumerate() {
            for slot in 0..4 {
                handles.insert(at + 12 + 8 * slot, silent);
            }
            for (k, value) in [1.0, 250.0, 0.0, 0.3, 249.0].into_iter().enumerate() {
                put(&mut block, at + 48 + 4 * k, value + n as f32);
            }
        }
        for (n, at) in WORLD_AT.into_iter().enumerate() {
            let slot = 396 + 8 * n;
            block[at + 16..at + 20].copy_from_slice(&(slot as u32).to_le_bytes());
            block[at + 32..at + 34].copy_from_slice(&1u16.to_le_bytes());
            handles.insert(slot, silent);
            put(&mut block, at + 20, 0.75);
        }
        for (n, at) in FLYBY_AT.into_iter().enumerate() {
            let slot = 428 + 16 * n;
            block[at + 16..at + 20].copy_from_slice(&(slot as u32).to_le_bytes());
            block[at + 24..at + 26].copy_from_slice(&2u16.to_le_bytes());
            handles.insert(slot, 0x7a25 + 2 * n as u64);
            handles.insert(slot + 8, 0x7a26 + 2 * n as u64);
            put(&mut block, at + 20, 37.5);
        }
        for n in 0..SET_FLOATS {
            put(&mut block, SET_FLOATS_AT + 4 * n, n as f32);
        }
        (block, handles)
    }

    #[test]
    fn the_layers_are_read_from_their_offsets() {
        let (block, handles) = sample_object(0x79ff);
        let set = SpeedSoundSet::parse(7, &block, &handles).unwrap();
        assert_eq!((set.wind.len(), set.world.len(), set.flyby.len(), set.values.len()), (2, 4, 2, 11));
        assert_eq!(set.wind[0].samples, [0x79ff; 4]);
        assert_eq!(set.wind[0].values, [1.0, 250.0, 0.0, 0.3, 249.0]);
        assert_eq!(set.wind[1].values[0], 2.0);
        assert_eq!(set.world[3].samples, [0x79ff]);
        assert_eq!(set.world[0].values[0], 0.75);
        assert_eq!((set.flyby[1].samples.clone(), set.flyby[1].value), (vec![0x7a27, 0x7a28], 37.5));
        assert_eq!(set.values[10], 10.0);
    }

    #[test]
    fn one_sample_in_every_wind_and_world_slot_is_a_placeholder() {
        let (block, mut handles) = sample_object(0x79ff);
        let set = SpeedSoundSet::parse(7, &block, &handles).unwrap();
        assert_eq!(set.placeholder(), Some(0x79ff));
        handles.insert(12 + 12, 0x1234);
        let other = SpeedSoundSet::parse(7, &block, &handles).unwrap();
        assert_eq!(other.placeholder(), None);
    }

    #[test]
    fn a_block_that_ends_early_is_not_read() {
        let (block, handles) = sample_object(1);
        assert!(SpeedSoundSet::parse(7, &block[..300], &handles).is_none());
    }
}
