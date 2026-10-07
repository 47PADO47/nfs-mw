//! A section's GPU resources and placed instances.

use std::collections::HashMap;

use blackbox_render::{BlendMode, MeshHandle, Renderer, Shading, TextureHandle};
use blackbox_scene::{Aabb, blend_mode, upload_solid, upload_texture};
use glam::Mat4;
use nfsmw_data::world::SectionData;

/// One placed copy of a mesh.
pub struct Placed {
    pub mesh: MeshHandle,
    pub transform: Mat4,
    /// World-space bounds (from the scenery instance).
    pub bounds: Aabb,
}

/// Textures and meshes of a section, by name hash.
#[derive(Default)]
pub struct SectionResources {
    pub materials: HashMap<u32, (TextureHandle, BlendMode)>,
    pub meshes: HashMap<u32, MeshHandle>,
}

impl SectionResources {
    pub fn upload_textures(&mut self, renderer: &mut Renderer, data: &SectionData) {
        for t in &data.textures {
            if self.materials.contains_key(&t.name_hash) {
                continue;
            }
            if let Some(handle) = upload_texture(renderer, t) {
                self.materials.insert(t.name_hash, (handle, blend_mode(Some(t))));
            }
        }
    }

    /// Upload solids, resolving textures here first and then in `fallback` (the shared sets).
    pub fn upload_meshes(&mut self, renderer: &mut Renderer, data: &SectionData, fallback: &SectionResources) {
        let lookup = |hash: u32| self.materials.get(&hash).or_else(|| fallback.materials.get(&hash)).copied();
        let mut meshes = Vec::new();
        for solid in &data.solids {
            if let Some(mesh) = upload_solid(renderer, solid, &lookup, Shading::Prelit) {
                meshes.push((solid.name_hash, mesh));
            }
        }
        self.meshes.extend(meshes);
    }

    pub fn mesh(&self, hash: u32, fallback: &SectionResources) -> Option<MeshHandle> {
        self.meshes.get(&hash).or_else(|| fallback.meshes.get(&hash)).copied()
    }

    pub fn release(self, renderer: &mut Renderer) {
        for mesh in self.meshes.into_values() {
            renderer.destroy_mesh(mesh);
        }
        for (texture, _) in self.materials.into_values() {
            renderer.destroy_texture(texture);
        }
    }
}

/// Turn a section's scenery into placed meshes. Sky domes are skipped: they
/// need to follow the camera and stay out of the fog.
pub fn place(data: &SectionData, own: &SectionResources, shared: &SectionResources) -> (Vec<Placed>, usize) {
    let mut placed = Vec::new();
    let mut unresolved = 0;
    for section in &data.scenery {
        for inst in &section.instances {
            let Some(info) = section.info_of(inst) else { continue };
            let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
            if info.name.starts_with("SKYDOME") || !inst.visible_in(rules.player_view, rules) {
                continue;
            }
            match info.best_solid().and_then(|key| own.mesh(key, shared)) {
                Some(mesh) => placed.push(Placed {
                    mesh,
                    transform: Mat4::from_cols_array_2d(&inst.matrix_columns()),
                    bounds: Aabb::new(inst.bbox_min, inst.bbox_max),
                }),
                None => unresolved += 1,
            }
        }
    }
    (placed, unresolved)
}
