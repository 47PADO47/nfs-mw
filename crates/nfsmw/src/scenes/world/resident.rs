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
        self.upload_texture_list(renderer, &data.textures);
    }

    /// Upload textures whose hash is not resident yet.
    pub fn upload_texture_list(&mut self, renderer: &mut Renderer, textures: &[blackbox_tpk::Texture]) {
        for t in textures {
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
            if let Some(mesh) = upload_solid(renderer, solid, &lookup, shading_for(&solid.name)) {
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

/// Sky domes (`SKYDOME`, `SKYDOME_XENON`, `SKY_SPECULAR`) are world-space models
/// around the whole map; they draw without fog.
fn shading_for(solid_name: &str) -> Shading {
    if solid_name.starts_with("SKY") { Shading::Sky } else { Shading::Prelit }
}

/// Scenery we can't draw correctly yet:
/// - `SKYDOME_XENON`: the next-gen sky dome, whose effect-19 / 44-byte vertex layout is not decoded;
/// - `SKY_SPECULAR`: a sky layer textured `SKY_REFSKYSPECULARB`, apparently for reflections.
const SKIPPED_MODELS: &[&str] = &["SKYDOME_XENON", "SKY_SPECULAR"];

/// Turn a section's scenery into placed meshes, dropping instances the player
/// view excludes (`docs/specs/scenery-visibility.md`).
pub fn place(data: &SectionData, own: &SectionResources, shared: &SectionResources) -> (Vec<Placed>, usize) {
    let mut placed = Vec::new();
    let mut unresolved = 0;
    for section in &data.scenery {
        for inst in &section.instances {
            let Some(info) = section.info_of(inst) else { continue };
            let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
            if !inst.visible_in(rules.player_view, rules) || SKIPPED_MODELS.contains(&info.name.as_str()) {
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
