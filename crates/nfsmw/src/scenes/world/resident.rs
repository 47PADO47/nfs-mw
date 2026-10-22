//! A section's GPU resources and placed instances.

use std::collections::HashMap;

use blackbox_gfx::{BlendMode, InstanceKey, MeshDesc, MeshHandle, RenderBackend, Shading, TextureHandle, Vertex};
use blackbox_scene::{Aabb, MaterialLookup, blend_mode, solid_mesh, upload_solid, upload_texture_predecoded_mips};
use blackbox_scenery::LodModel;
use blackbox_solid::Solid;
use glam::Mat4;
use nfsmw_data::world::{PropShape, SectionData};
use std::sync::Arc;
mod prepare;
mod reflections;
mod shadow_policy;
pub(super) use prepare::{resolve_placed, take_prepared, worker_hook};
pub(super) use reflections::{ReflectionPlaced, resolve_reflections};
#[cfg(test)]
mod group_tests;
#[cfg(test)]
mod mesh_lifetime_tests;

/// One placed scenery object: up to four LOD meshes, chosen per frame
/// (`docs/specs/scenery-lod.md`).
#[derive(Clone)]
pub struct Placed {
    pub lods: [Option<MeshHandle>; 4],
    /// The slot-0 solid's polygon count and density, for the LOD choice.
    pub detailed: Option<LodModel>,
    pub transform: Mat4,
    /// World-space bounds (from the scenery instance).
    pub bounds: Aabb,
    /// `SceneryInfo::radius`.
    pub radius: f32,
    pub flags: u32,
    /// The scenery group that hides this instance, or 0. A grouped instance is kept while hidden and drawn only
    /// once its group is enabled (`visibility.rs`).
    pub group: u16,
    /// The collision of this scenery object, if the track has bounds for it.
    pub prop: Option<Arc<PropShape>>,
    /// The id the prop world gave it (0 until the tile is registered).
    pub prop_id: u32,
    /// Its name for the renderer, from the tile and its place in the tile's list (see [`super::keys`]).
    pub key: InstanceKey,
    /// Authored shadow-only scenery, identified in this game's asset names.
    pub original_shadow: bool,
    /// Physical sun occluder; independent of background colour placement.
    pub casts_sun_shadow: bool,
    /// Original physics pose and stable identity, when this instance has a spawner.
    pub spawner: Option<blackbox_scenery::ObjectSpawner>,
    pub parts: Option<Arc<super::props::scenery::ModelParts>>,
    /// The flashing lamp on a curve warning sign: its place in the world, or `None` for other objects.
    pub curve_lamp: Option<glam::Vec3>,
    /// The lamp head of a real traffic light (`ai/signal_props.rs`), or `None` for other objects.
    pub signal: Option<super::ai::signal_props::SignalHead>,
}

#[cfg(test)]
impl Placed {
    /// A placed object with collision only, for tests.
    pub fn prop_only(transform: Mat4, shape: Arc<PropShape>) -> Self {
        let at = transform.transform_point3(glam::Vec3::ZERO);
        Self {
            group: 0,
            lods: [None; 4],
            detailed: None,
            transform,
            bounds: Aabb::new((at - 1.0).to_array(), (at + 1.0).to_array()),
            radius: 1.0,
            flags: 0,
            prop: Some(shape),
            prop_id: 0,
            key: InstanceKey::TRANSIENT,
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            curve_lamp: None,
            signal: None,
        }
    }
}

/// Textures and meshes of a section, by name hash.
#[derive(Default)]
pub struct SectionResources {
    pub(super) texture_discard: u32,
    /// Authored road-view placements; never mixed into the player or shadow feed.
    pub(super) reflection_placed: Vec<ReflectionPlaced>,
    pub(super) normal_materials:
        HashMap<(blackbox_gfx::NormalModel, [TextureHandle; 3]), blackbox_gfx::NormalMaterialHandle>,
    pub(super) window_materials: HashMap<[TextureHandle; 2], blackbox_gfx::WindowMaterialHandle>,
    pub materials: HashMap<u32, (TextureHandle, BlendMode)>,
    pub meshes: HashMap<u32, (MeshHandle, LodModel, bool)>,
    /// Animated textures defined here; frames may live here or in the shared sets.
    pub anims: Vec<blackbox_tpk::TextureAnim>,
    /// The frame (its texture hash) each animation in `anims` was last redirected to, so a frame that is still
    /// showing costs no renderer call. Filled by `animate`.
    shown_frames: std::cell::RefCell<Vec<Option<u32>>>,
}

impl SectionResources {
    pub(super) fn with_texture_detail(texture_discard: u32) -> Self {
        Self { texture_discard, ..Default::default() }
    }

    pub fn upload_textures(&mut self, renderer: &mut dyn RenderBackend, data: &SectionData) {
        self.upload_texture_list(renderer, &data.textures, &data.decoded);
    }

    /// Upload textures whose hash is not resident yet. `decoded` is, per texture, the RGBA8 a loader thread made
    /// (`SectionData::decoded`); it is empty when none were made.
    pub fn upload_texture_list(
        &mut self,
        renderer: &mut dyn RenderBackend,
        textures: &[blackbox_tpk::Texture],
        decoded: &[Option<Vec<u8>>],
    ) {
        for (i, t) in textures.iter().enumerate() {
            if self.materials.contains_key(&t.name_hash) {
                continue;
            }
            let pixels = decoded.get(i).and_then(Option::as_deref);
            let discard = match t.name.starts_with("SKY") {
                true => 0,
                false => self.texture_discard,
            };
            if let Some(handle) = upload_texture_predecoded_mips(renderer, t, pixels, discard) {
                self.materials.insert(t.name_hash, (handle, blend_mode(Some(t))));
            }
        }
    }

    /// Upload solids, resolving textures here first and then in `fallback` (the shared sets).
    pub fn upload_meshes(&mut self, renderer: &mut dyn RenderBackend, data: &SectionData, fallback: &SectionResources) {
        self.upload_solid_list(renderer, &data.solids, Some(&data.solid_vertices), fallback, false);
    }

    /// Upload solids, resolving textures here first and then in `fallback`. `vertices` are the solids' renderer
    /// vertices in the same order, built on the loader thread; without them each solid is converted here. With
    /// `keep_existing`, a solid whose name hash is already resident is skipped (the first upload wins, as the
    /// textures do).
    pub fn upload_solid_list(
        &mut self,
        renderer: &mut dyn RenderBackend,
        solids: &[Solid],
        vertices: Option<&[Vec<Vertex>]>,
        fallback: &SectionResources,
        keep_existing: bool,
    ) {
        debug_assert!(vertices.is_none_or(|v| v.len() == solids.len()));
        let lookup = |hash: u32| self.materials.get(&hash).or_else(|| fallback.materials.get(&hash)).copied();
        let mut meshes = Vec::new();
        for (i, solid) in solids.iter().enumerate() {
            if keep_existing && self.meshes.contains_key(&solid.name_hash) {
                continue;
            }
            if is_tunnel_shadow(&solid.name) {
                continue;
            }
            let mut normal_materials =
                super::materials::normal_materials(renderer, solid, &lookup, &mut self.normal_materials);
            let windows = super::materials::window_materials(renderer, solid, &lookup, &mut self.window_materials);
            for (material, window) in normal_materials.iter_mut().zip(windows) {
                if window.is_some() {
                    *material = window;
                }
            }
            let materials = super::materials::Lookup(&lookup, solid.name.starts_with("SKY"), normal_materials);
            let shading = shading_for(&solid.name);
            let mesh = match vertices {
                Some(prepared) => upload_prepared(renderer, solid, &prepared[i], &materials, shading),
                None => upload_solid(renderer, solid, &materials, shading),
            };
            if let Some(mesh) = mesh {
                let lod = LodModel { num_polys: solid.num_polys, density: solid.density };
                meshes.push((solid.name_hash, (mesh, lod, solid.name.starts_with("SHD_"))));
            }
        }
        for (hash, mesh) in meshes {
            if let Some(previous) = self.meshes.insert(hash, mesh) {
                renderer.destroy_mesh(previous.0);
            }
        }
    }

    /// Show each animation's current frame (`seconds` since the scene started). An animation whose frame has not
    /// changed since the last call is left alone.
    pub fn animate(&self, renderer: &mut dyn RenderBackend, seconds: f32, fallback: &SectionResources) {
        let texture = |hash: u32| self.materials.get(&hash).or_else(|| fallback.materials.get(&hash)).map(|m| m.0);
        let mut shown = self.shown_frames.borrow_mut();
        shown.resize(self.anims.len(), None);
        for (anim, shown) in self.anims.iter().zip(shown.iter_mut()) {
            let Some(frame) = anim.frame_at(seconds) else { continue };
            if *shown == Some(frame) {
                continue;
            }
            if let (Some(base), Some(to)) = (texture(anim.name_hash), texture(frame)) {
                renderer.redirect_texture(base, Some(to));
                *shown = Some(frame);
            }
        }
    }

    pub fn mesh(&self, hash: u32, fallback: &SectionResources) -> Option<(MeshHandle, LodModel)> {
        self.meshes.get(&hash).or_else(|| fallback.meshes.get(&hash)).map(|&(mesh, lod, _)| (mesh, lod))
    }

    pub(super) fn release_normal_materials(&mut self, renderer: &mut dyn RenderBackend) {
        for material in self.normal_materials.drain().map(|(_, handle)| handle) {
            renderer.destroy_normal_material(material);
        }
        for material in self.window_materials.drain().map(|(_, handle)| handle) {
            renderer.destroy_window_material(material);
        }
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for (mesh, _, _) in self.meshes.into_values() {
            renderer.destroy_mesh(mesh);
        }
        for material in self.normal_materials.into_values() {
            renderer.destroy_normal_material(material);
        }
        for material in self.window_materials.into_values() {
            renderer.destroy_window_material(material);
        }
        for (texture, _) in self.materials.into_values() {
            renderer.destroy_texture(texture);
        }
    }
}

/// Upload a solid whose renderer vertices were built ahead of time. Only the draw ranges are made here, since
/// they need the textures this thread has uploaded. Returns `None` for solids without geometry.
fn upload_prepared(
    renderer: &mut dyn RenderBackend,
    solid: &Solid,
    vertices: &[Vertex],
    materials: &impl MaterialLookup,
    shading: Shading,
) -> Option<MeshHandle> {
    if vertices.is_empty() || solid.indices.is_empty() {
        return None;
    }
    let draws = solid_mesh(solid, materials, shading);
    Some(renderer.create_mesh(&MeshDesc {
        label: &solid.name,
        vertices,
        secondary_uvs: Some(&solid.secondary_uvs),
        tangents: Some(&solid.tangents),
        indices: &solid.indices,
        draws,
    }))
}

/// Sky domes (`SKYDOME`, `SKYDOME_XENON`, `SKY_SPECULAR`) are world-space models
/// around the whole map; they draw without fog.
fn shading_for(solid_name: &str) -> Shading {
    if solid_name.starts_with("SKY") { Shading::Sky } else { Shading::Prelit }
}

/// Scenery we can't draw correctly yet: `SKY_SPECULAR`, a sky layer textured
/// `SKY_REFSKYSPECULARB` that darkens the whole dome in the player view (apparently for reflections).
const SKIPPED_MODELS: &[&str] = &["SKY_SPECULAR"];

/// Tunnel shadow solids (`SHADOW_TUNNELWALLB01_*`, `SHADOW_TUNNELROOFA01`). Their copy-mode textures
/// decode to opaque near-black, not to a shadow alpha, so drawn they blank out whole tunnel walls.
/// They are skipped until the game's shadow blend is decoded.
fn is_tunnel_shadow(solid_name: &str) -> bool {
    solid_name.starts_with("SHADOW_")
}

/// The scenery group that keeps a hidden instance: its group when `hidden` and a group lists its override, else 0.
/// A visible instance is never tagged, so a group only brings back what it hides.
fn hidden_group(hidden: bool, override_index: Option<u32>, groups: &HashMap<u16, u16>) -> u16 {
    if !hidden {
        return 0;
    }
    override_index
        .and_then(|index| u16::try_from(index).ok())
        .and_then(|index| groups.get(&index).copied())
        .unwrap_or(0)
}

/// Turn a section's scenery into placed meshes, dropping instances the player
/// view excludes (`docs/specs/scenery-visibility.md`). An instance the player view
/// excludes but a scenery group lists is kept, tagged with the group's number.
/// `groups` maps a global override index to its group number (`WorldIndex::override_groups`).
///
/// The residency does this in two steps so a loader thread can do the first (`prepare.rs`); this runs both.
#[cfg(test)]
pub fn place(
    data: &SectionData,
    own: &SectionResources,
    shared: &SectionResources,
    props: &nfsmw_data::world::PropCatalog,
    groups: &HashMap<u16, u16>,
) -> (Vec<Placed>, usize) {
    resolve_placed(&prepare::prepare(data, props, groups).placed, own, shared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_shadow_solids_are_not_uploaded_but_other_overlays_are() {
        assert!(is_tunnel_shadow("SHADOW_TUNNELWALLB01_DEINST1_I4"));
        assert!(is_tunnel_shadow("SHADOW_TUNNELROOFA01"));
        assert!(!is_tunnel_shadow("SHD_CP_BE_ROAD_A_02A"));
        assert!(!is_tunnel_shadow("TRN_CT_Road_Highway4_XX"));
    }
}
