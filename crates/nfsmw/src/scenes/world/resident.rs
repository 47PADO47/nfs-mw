//! A section's GPU resources and placed instances.

use std::collections::HashMap;

use blackbox_gfx::{BlendMode, InstanceKey, MeshDesc, MeshHandle, RenderBackend, Shading, TextureHandle, Vertex};
use blackbox_scene::{Aabb, MaterialLookup, blend_mode, solid_mesh, upload_solid, upload_texture};
use blackbox_scenery::LodModel;
use blackbox_solid::Solid;
use glam::Mat4;
use nfsmw_data::world::{PropCatalog, PropShape, SectionData};
use std::sync::Arc;
mod reflections;
mod shadow_policy;
pub(super) use reflections::{ReflectionPlaced, place_reflections};
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
    /// Authored road-view placements; never mixed into the player or shadow feed.
    pub(super) reflection_placed: Vec<ReflectionPlaced>,
    pub(super) normal_materials:
        HashMap<(blackbox_gfx::NormalModel, [TextureHandle; 3]), blackbox_gfx::NormalMaterialHandle>,
    pub(super) window_materials: HashMap<[TextureHandle; 2], blackbox_gfx::WindowMaterialHandle>,
    pub materials: HashMap<u32, (TextureHandle, BlendMode)>,
    pub meshes: HashMap<u32, (MeshHandle, LodModel, bool)>,
    /// Animated textures defined here; frames may live here or in the shared sets.
    pub anims: Vec<blackbox_tpk::TextureAnim>,
}

impl SectionResources {
    pub fn upload_textures(&mut self, renderer: &mut dyn RenderBackend, data: &SectionData) {
        self.upload_texture_list(renderer, &data.textures);
    }

    /// Upload textures whose hash is not resident yet.
    pub fn upload_texture_list(&mut self, renderer: &mut dyn RenderBackend, textures: &[blackbox_tpk::Texture]) {
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

    /// Show each animation's current frame (`seconds` since the scene started).
    pub fn animate(&self, renderer: &mut dyn RenderBackend, seconds: f32, fallback: &SectionResources) {
        let texture = |hash: u32| self.materials.get(&hash).or_else(|| fallback.materials.get(&hash)).map(|m| m.0);
        for anim in &self.anims {
            if let (Some(base), Some(frame)) = (texture(anim.name_hash), anim.frame_at(seconds).and_then(texture)) {
                renderer.redirect_texture(base, Some(frame));
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

/// Turn a section's scenery into placed meshes, dropping instances the player
/// view excludes (`docs/specs/scenery-visibility.md`).
pub fn place(
    data: &SectionData,
    own: &SectionResources,
    shared: &SectionResources,
    props: &PropCatalog,
) -> (Vec<Placed>, usize) {
    let instances: usize = data.scenery.iter().map(|s| s.instances.len()).sum();
    let mut placed = Vec::with_capacity(instances);
    let mut unresolved = 0;
    let mut spawners = HashMap::with_capacity(data.spawners.len());
    spawners.extend(data.spawners.iter().map(|s| (s.override_index, s)));
    for section in &data.scenery {
        let mut hooks = HashMap::with_capacity(section.override_hooks.len());
        hooks.extend(
            section.override_hooks.iter().map(|h| (usize::from(h.instance_index), u32::from(h.override_index))),
        );
        for (instance, inst) in section.instances.iter().enumerate() {
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
            let transform = Mat4::from_cols_array_2d(&inst.matrix_columns());
            let spawner = hooks.get(&instance).and_then(|id| spawners.get(id)).copied();
            // Static solidity comes from authored world collision, not a model-name bounds guess.
            let prop = spawner.and_then(|spawn| props.for_spawner(&info.name, spawn));
            if spawner.is_some() && prop.is_none() {
                log::debug!("no physics bounds/attributes for placed {}", info.name);
            }
            placed.push(Placed {
                casts_sun_shadow: shadow_policy::casts_sun(&info.name),
                original_shadow: info.name.starts_with("SHD_")
                    || resolved.iter().enumerate().filter(|(_, mesh)| mesh.is_some()).all(|(i, _)| {
                        own.meshes
                            .get(&info.solid_keys[i])
                            .or_else(|| shared.meshes.get(&info.solid_keys[i]))
                            .is_some_and(|model| model.2)
                    }),
                lods: std::array::from_fn(|i| resolved.get(i).copied().flatten().map(|(m, _)| m)),
                detailed: resolved.first().copied().flatten().map(|(_, lod)| lod),
                transform,
                bounds: Aabb::new(inst.bbox_min, inst.bbox_max),
                radius: info.radius,
                flags: inst.exclude_flags,
                prop,
                prop_id: 0,
                key: InstanceKey::TRANSIENT,
                spawner: spawner.copied(),
                curve_lamp: super::sign_lamps::lamp_point(&info.name, Aabb::new(inst.bbox_min, inst.bbox_max)),
                signal: super::ai::signal_props::signal_head(&info.name, transform),
                parts: props.hierarchy(info.hierarchy_hash).map(|hierarchy| {
                    let meshes: std::collections::BTreeMap<_, _> = hierarchy
                        .nodes
                        .iter()
                        .enumerate()
                        .filter_map(|(i, n)| Some((i, own.mesh(n.model_hash, shared)?.0)))
                        .collect();
                    Arc::new(super::props::scenery::ModelParts {
                        available: meshes.keys().copied().collect(),
                        hierarchy,
                        meshes,
                    })
                }),
            });
        }
    }
    (placed, unresolved)
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
