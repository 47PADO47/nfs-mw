//! Scenery collision and original physics attributes, joined by serialized keys.
//! Specs: `docs/formats/prop-spawners.md`, `docs/specs/prop-attributes.md`.

mod attributes;
mod shapes;
pub use attributes::PropDynamics;
pub use shapes::boxes;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use blackbox_attrib::{Database, vlt_hash};
use blackbox_collision::BoundsSet;
use blackbox_scenery::ObjectSpawner;
use glam::{Quat, Vec3};

/// Name fallback for static scenery whose original full name was truncated.
const NAME_ENDINGS: [&str; 6] = ["0", "00", "_DE", "_DE0", "1", "A"];

/// A box in model axes (x forward, y left, z up), used for static scenery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalBox {
    pub centre: Vec3,
    pub rotation: Quat,
    pub half: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropKind {
    Rigid,
    /// Movable regardless of mass; original attributes decide attachment and response.
    Movable {
        mass: f32,
    },
}

#[derive(Debug, Clone)]
pub struct PropShape {
    pub name: String,
    pub boxes: Vec<LocalBox>,
    pub kind: PropKind,
    /// Exact source bounds in physics axes, including primitive flags and hierarchy.
    pub bounds: Option<Arc<BoundsSet>>,
    pub dynamics: Option<Arc<PropDynamics>>,
}

/// Track bounds and inherited prop attributes. Names never select a dynamic object's behavior.
pub struct PropCatalog {
    sets: HashMap<u32, Arc<BoundsSet>>,
    dynamics: HashMap<u32, Arc<PropDynamics>>,
    cache: RefCell<HashMap<String, Option<Arc<PropShape>>>>,
    placed_cache: RefCell<HashMap<(u32, u32, String), Arc<PropShape>>>,
}

impl PropCatalog {
    pub fn new(sets: Vec<BoundsSet>, db: &Database) -> Self {
        let dynamics =
            db.collections_of("smackable").filter_map(|c| Some((c.key(), Arc::new(PropDynamics::read(c)?)))).collect();
        Self {
            sets: sets.into_iter().map(|s| (s.name_hash, Arc::new(s))).collect(),
            dynamics,
            cache: RefCell::default(),
            placed_cache: RefCell::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// Static fallback only. A missing spawner must not turn a name guess into a movable object.
    pub fn shape(&self, name: &str) -> Option<Arc<PropShape>> {
        if let Some(known) = self.cache.borrow().get(name) {
            return known.clone();
        }
        let set = std::iter::once("")
            .chain(NAME_ENDINGS)
            .find_map(|ending| self.sets.get(&vlt_hash(&format!("{name}{ending}"))));
        let shape = set
            .map(|set| {
                Arc::new(PropShape {
                    name: name.to_owned(),
                    boxes: boxes(set),
                    kind: PropKind::Rigid,
                    bounds: None,
                    dynamics: None,
                })
            })
            .filter(|s| !s.boxes.is_empty());
        self.cache.borrow_mut().insert(name.to_owned(), shape.clone());
        shape
    }

    /// Exact collision and attribute keys. Missing bounds/attributes stay missing.
    pub fn for_spawner(&self, name: &str, spawn: &ObjectSpawner) -> Option<Arc<PropShape>> {
        // Distinct visible models can share bounds and attributes. Their identity is still distinct.
        let key = (spawn.collision, spawn.attributes, name.to_owned());
        if let Some(shape) = self.placed_cache.borrow().get(&key) {
            return Some(shape.clone());
        }
        let bounds = self.sets.get(&spawn.collision)?.clone();
        let dynamics = self.dynamics.get(&spawn.attributes)?.clone();
        let kind = match dynamics.immobile {
            true => PropKind::Rigid,
            false => PropKind::Movable { mass: dynamics.mass },
        };
        let shape = Arc::new(PropShape {
            name: name.to_owned(),
            boxes: boxes(&bounds),
            kind,
            bounds: Some(bounds),
            dynamics: Some(dynamics),
        });
        self.placed_cache.borrow_mut().insert(key, shape.clone());
        Some(shape)
    }
}
