//! A section's GPU resources and placed instances.

use std::collections::HashMap;

use blackbox_render::{BlendMode, MeshHandle, Renderer, Shading, TextureHandle};
use blackbox_scene::{Aabb, blend_mode, upload_solid, upload_texture};
use blackbox_scenery::LodModel;
use glam::Mat4;
use nfsmw_data::world::{PropCatalog, PropShape, SectionData};
use std::sync::Arc;

/// One placed scenery object: up to four LOD meshes, chosen per frame
/// (`docs/specs/scenery-lod.md`).
pub struct Placed {
    pub lods: [Option<MeshHandle>; 4],
    /// The slot-0 solid's polygon count and density, for the LOD choice.
    pub detailed: Option<LodModel>,
    pub transform: Mat4,
    /// World-space bounds (from the scenery instance).
    pub bounds: Aabb,
    pub position: [f32; 3],
    /// `SceneryInfo::radius`.
    pub radius: f32,
    pub flags: u32,
    /// The collision of this scenery object, if the track has bounds for it.
    pub prop: Option<Arc<PropShape>>,
    /// The id the prop world gave it (0 until the tile is registered).
    pub prop_id: u32,
}

#[cfg(test)]
impl Placed {
    /// A placed object with collision only, for tests.
    pub fn prop_only(transform: Mat4, shape: Arc<PropShape>) -> Self {
        let at = transform.transform_point3(glam::Vec3::ZERO);
        Self {
            lods: [None; 4],
            detailed: None,
            transform,
            bounds: Aabb::new((at - 1.0).to_array(), (at + 1.0).to_array()),
            position: at.to_array(),
            radius: 1.0,
            flags: 0,
            prop: Some(shape),
            prop_id: 0,
        }
    }
}

/// Textures and meshes of a section, by name hash.
#[derive(Default)]
pub struct SectionResources {
    pub materials: HashMap<u32, (TextureHandle, BlendMode)>,
    pub meshes: HashMap<u32, (MeshHandle, LodModel)>,
    /// Animated textures defined here; frames may live here or in the shared sets.
    pub anims: Vec<blackbox_tpk::TextureAnim>,
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
                let lod = LodModel { num_polys: solid.num_polys, density: solid.density };
                meshes.push((solid.name_hash, (mesh, lod)));
            }
        }
        self.meshes.extend(meshes);
    }

    /// Show each animation's current frame (`seconds` since the scene started).
    pub fn animate(&self, renderer: &mut Renderer, seconds: f32, fallback: &SectionResources) {
        let texture = |hash: u32| self.materials.get(&hash).or_else(|| fallback.materials.get(&hash)).map(|m| m.0);
        for anim in &self.anims {
            if let (Some(base), Some(frame)) = (texture(anim.name_hash), anim.frame_at(seconds).and_then(texture)) {
                renderer.redirect_texture(base, Some(frame));
            }
        }
    }

    pub fn mesh(&self, hash: u32, fallback: &SectionResources) -> Option<(MeshHandle, LodModel)> {
        self.meshes.get(&hash).or_else(|| fallback.meshes.get(&hash)).copied()
    }

    pub fn release(self, renderer: &mut Renderer) {
        for (mesh, _) in self.meshes.into_values() {
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

/// Scenery we can't draw correctly yet: `SKY_SPECULAR`, a sky layer textured
/// `SKY_REFSKYSPECULARB` that darkens the whole dome in the player view (apparently for reflections).
const SKIPPED_MODELS: &[&str] = &["SKY_SPECULAR"];

/// Turn a section's scenery into placed meshes, dropping instances the player
/// view excludes (`docs/specs/scenery-visibility.md`).
pub fn place(
    data: &SectionData,
    own: &SectionResources,
    shared: &SectionResources,
    props: &PropCatalog,
) -> (Vec<Placed>, usize) {
    let mut placed = Vec::new();
    let mut unresolved = 0;
    for section in &data.scenery {
        for inst in &section.instances {
            let Some(info) = section.info_of(inst) else { continue };
            let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
            if !inst.visible_in(rules.player_view, rules) || SKIPPED_MODELS.contains(&info.name.as_str()) {
                continue;
            }
            let resolved: Vec<_> =
                info.solid_keys.iter().map(|&k| (k != 0).then(|| own.mesh(k, shared)).flatten()).collect();
            if resolved.iter().all(Option::is_none) {
                unresolved += 1;
                continue;
            }
            placed.push(Placed {
                lods: std::array::from_fn(|i| resolved.get(i).copied().flatten().map(|(m, _)| m)),
                detailed: resolved.first().copied().flatten().map(|(_, lod)| lod),
                transform: Mat4::from_cols_array_2d(&inst.matrix_columns()),
                bounds: Aabb::new(inst.bbox_min, inst.bbox_max),
                position: inst.position,
                radius: info.radius,
                flags: inst.exclude_flags,
                prop: props.shape(&info.name),
                prop_id: 0,
            });
        }
    }
    (placed, unresolved)
}
