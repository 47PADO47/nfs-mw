//! The parts of a model the run-time props release, with the meshes the shared sets hold.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Residency, TileState};
use crate::scenes::world::props::scenery::ModelParts;

impl Residency {
    /// Apply the two continuously rotating authored solids to already collected scenery.
    pub(in crate::scenes::world) fn animate_simple_models(
        &self,
        instances: &mut [blackbox_gfx::Instance],
        seconds: f32,
    ) {
        use blackbox_hash::bstring_hash;
        use glam::Mat4;
        let names = [("XB_DONUTSIGNB_1B_BK_00", false), ("XO_WINDMILL_BLADE_1B_JG_00", true)];
        let mut meshes = std::collections::HashMap::new();
        for (name, around_y) in names {
            let hash = bstring_hash(name);
            if let Some((mesh, _, _)) = self.shared.meshes.get(&hash) {
                meshes.insert(*mesh, around_y);
            }
            for state in self.tiles.values() {
                if let TileState::Resident { resources, .. } = state
                    && let Some((mesh, _, _)) = resources.meshes.get(&hash)
                {
                    meshes.insert(*mesh, around_y);
                }
            }
        }
        let angle = (seconds * 50.0).to_radians();
        for instance in instances {
            if let Some(around_y) = meshes.get(&instance.mesh) {
                let turn = if *around_y { Mat4::from_rotation_y(angle) } else { Mat4::from_rotation_z(angle) };
                instance.transform *= turn;
            }
        }
    }
    pub(in crate::scenes::world) fn animation_section_visible(&self, section: i32) -> bool {
        self.sections.iter().enumerate().any(|(index, item)| {
            if i32::from(item.number) != section {
                return false;
            }
            if !item.is_spatial() {
                return self.shared_ready;
            }
            (self.drawn.contains(&index) || self.previous.contains(&index))
                && matches!(self.tiles.get(&index), Some(TileState::Resident { .. }))
        })
    }

    pub(in crate::scenes::world) fn animation_mesh(
        &self,
        section: i32,
        model: u32,
    ) -> Option<blackbox_gfx::MeshHandle> {
        if let Some(mesh) = self.shared.meshes.get(&model) {
            return Some(mesh.0);
        }
        self.sections.iter().enumerate().filter(|(_, item)| i32::from(item.number) == section).find_map(|(index, _)| {
            match self.tiles.get(&index) {
                Some(TileState::Resident { resources, .. }) => resources.meshes.get(&model).map(|mesh| mesh.0),
                _ => None,
            }
        })
    }
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
