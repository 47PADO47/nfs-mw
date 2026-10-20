//! The parts of a model the run-time props release, with the meshes the shared sets hold.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::Residency;
use crate::scenes::world::props::scenery::ModelParts;

impl Residency {
    /// The hierarchy of the model called `name` with the meshes of the shared sets: the parts a run-time prop's
    /// sequence releases (a roadblock's sawhorse). `None` while the shared sets or the hierarchy are missing.
    pub fn fragment_parts(&self, name: &str) -> Option<Arc<ModelParts>> {
        let hierarchy = self.catalog.hierarchy_of(name)?;
        let meshes: BTreeMap<_, _> = hierarchy
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| Some((i, self.shared.meshes.get(&n.model_hash)?.0)))
            .collect();
        (!meshes.is_empty())
            .then(|| Arc::new(ModelParts { available: meshes.keys().copied().collect(), hierarchy, meshes }))
    }
}
