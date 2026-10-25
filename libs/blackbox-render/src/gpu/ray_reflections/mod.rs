//! Optional Vulkan one-bounce reflection scene. See docs/vulkan-ray-reflections.md.
mod arena;
mod atlas;
pub(super) mod bindings;
mod cache;
mod mesh;
mod ranges;
mod surfaces;
#[cfg(test)]
mod tests;

use super::Renderer;
use blackbox_gfx::{Instance, MeshDesc, Shading};
use std::collections::{BTreeMap, HashMap};

pub(super) use bindings::Bindings;
pub(super) use bindings::layout_entries;
#[allow(unsafe_code)]
pub(super) fn experimental_features(enabled: bool) -> wgpu::ExperimentalFeatures {
    if !enabled {
        return wgpu::ExperimentalFeatures::disabled();
    }
    // SAFETY: Only an explicit prototype opt-in enables wgpu's experimental APIs.
    // Geometry/ranges are owned and validated here; wgpu's documented implementation risk remains.
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}
pub(super) struct Scene {
    meshes: HashMap<usize, mesh::Mesh>,
    atlas: atlas::Atlas,
    detail: atlas::Atlas,
    detail_textures: std::collections::HashSet<usize>,
    quality: u32,
    camera: glam::Vec3,
    ranked_at: Option<glam::Vec3>,
    pub bindings: Bindings,
    selected: Vec<usize>,
    arena: arena::Arena,
    offscreen: Vec<Instance>,
    previous: Vec<(usize, [u32; 16], bool)>,
    live_count: usize,
    dirty: bool,
}
impl Scene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY) {
            return None;
        }
        let atlas = atlas::Atlas::new(device);
        let detail = atlas::Atlas::detail(device);
        let bindings = Bindings::new(device, &atlas, &detail);
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.build_acceleration_structures(std::iter::empty(), [&bindings.tlas]);
        queue.submit([encoder.finish()]);
        Some(Self {
            meshes: HashMap::new(),
            atlas,
            detail,
            detail_textures: Default::default(),
            quality: 1,
            camera: glam::Vec3::ZERO,
            ranked_at: None,
            bindings,
            selected: Vec::new(),
            arena: arena::Arena::default(),
            offscreen: Vec::new(),
            previous: Vec::new(),
            live_count: 0,
            dirty: true,
        })
    }
    pub(in crate::gpu) fn set_material(&self, queue: &wgpu::Queue, handle: usize, packed: Option<&[u8]>) {
        if handle >= bindings::MAX_MATERIALS {
            return;
        }
        let mut bytes = [0u8; bindings::MATERIAL_STRIDE];
        if let Some(packed) = packed {
            assert_eq!(packed.len(), 96);
            bytes[..96].copy_from_slice(packed);
            bytes[96..100].copy_from_slice(&1.0f32.to_le_bytes());
        }
        queue.write_buffer(&self.bindings.materials, (handle * bindings::MATERIAL_STRIDE) as u64, &bytes);
    }
    pub fn remove_mesh(&mut self, handle: usize) {
        self.arena.remove(handle);
        self.meshes.remove(&handle);
        self.dirty = true;
    }
    pub fn remove_texture(&mut self, handle: usize) {
        self.atlas.forget(handle);
        self.detail.forget(handle);
        self.detail_textures.remove(&handle);
        for (id, mesh) in &self.meshes {
            if mesh.textures.contains(&handle) {
                self.arena.remove(*id);
            }
        }
        self.dirty = true;
    }
    pub fn set_instances(&mut self, instances: &[Instance]) {
        self.offscreen.clear();
        self.offscreen.extend_from_slice(instances);
    }
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        main: &[Instance],
        views: &HashMap<usize, wgpu::TextureView>,
    ) -> bool {
        // Exact mesh/transform identity merges main and offscreen feeds without losing repeated placements.
        let mut unique = BTreeMap::new();
        for i in self.offscreen.iter().chain(main) {
            if !self.meshes.contains_key(&i.mesh.raw()) {
                continue;
            }
            if !i.transform.is_finite() || i.transform.determinant().abs() < 1e-12 {
                continue;
            }
            unique.insert((i.mesh.raw(), i.transform.to_cols_array().map(f32::to_bits), i.flip_winding), i);
        }
        let ids: Vec<_> = unique.keys().map(|k| k.0).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let keys: Vec<_> = unique.keys().copied().collect();
        if self.dirty
            || self.selected != ids
            || self.ranked_at.is_none_or(|p| p.distance_squared(self.camera) > 32.0 * 32.0)
        {
            let active = ids.iter().flat_map(|id| self.meshes[id].textures.iter().copied()).chain([0]).collect();
            self.atlas.retain(&active);
            let selected = cache::select(
                &self.meshes,
                unique.values().copied(),
                views,
                self.camera,
                self.detail.capacity(),
                &self.detail_textures,
            );
            let changed: std::collections::HashSet<_> =
                self.detail_textures.symmetric_difference(&selected).copied().collect();
            if !changed.is_empty() {
                for (id, mesh) in &self.meshes {
                    if mesh.textures.iter().any(|h| changed.contains(h)) {
                        self.arena.remove(*id);
                    }
                }
                self.dirty = true;
            }
            self.detail.retain(&selected);
            self.detail_textures = selected;
            self.ranked_at = Some(self.camera);
        }
        if !self.dirty && keys == self.previous {
            blackbox_trace::global().counter(
                "Vulkan RT scene",
                &[
                    ("instances", self.live_count as f64),
                    ("meshes", self.arena.len() as f64),
                    ("reused", 1.0),
                    ("detail_textures", self.detail.len() as f64),
                    ("texture_MiB", (self.atlas.bytes() + self.detail.bytes()) as f64 / 1048576.0),
                ],
            );
            return false;
        }
        self.previous = keys;
        let mut changed = false;
        let capacity = unique.len().max(1).next_power_of_two().min(bindings::MAX_INSTANCES);
        if capacity > self.bindings.tlas.get().len() {
            self.bindings.tlas = bindings::tlas(device, capacity);
            changed = true;
        }
        if self.dirty || self.selected != ids {
            let _scope = blackbox_trace::scope("Vulkan RT: update hit tables");
            self.arena.retain(&ids);
            let all_fit = ids.iter().all(|id| {
                self.arena.ensure(
                    *id,
                    &self.meshes[id],
                    device,
                    queue,
                    (&mut self.atlas, &mut self.detail, &self.detail_textures),
                    views,
                )
            });
            if !all_fit {
                // Fragmentation can exhaust the high-water mark even when live data fits.
                // Compact only in that exceptional case, retaining the prototype capacity policy.
                self.arena = arena::Arena::default();
                for id in &ids {
                    if !self.arena.ensure(
                        *id,
                        &self.meshes[id],
                        device,
                        queue,
                        (&mut self.atlas, &mut self.detail, &self.detail_textures),
                        views,
                    ) {
                        log::warn!("RT hit-shading buffer limit reached; excess meshes use environment fallback");
                        break;
                    }
                }
            }
            changed |= self.arena.flush(device, queue, &mut self.bindings);
            self.selected = ids;
            self.dirty = false;
        }
        let capacity = self.bindings.tlas.get().len();
        self.bindings.tlas.get_mut_slice(0..capacity).unwrap().fill(None);
        let mut count = 0;
        let mut winding = Vec::with_capacity(unique.len().min(bindings::MAX_INSTANCES));
        for id in self.arena.ids() {
            self.meshes.get_mut(id).unwrap().build(device, queue);
        }
        for i in unique.values() {
            let Some(offset) = self.arena.offset(i.mesh.raw()) else { continue };
            if count == bindings::MAX_INSTANCES {
                break;
            }
            let m = &self.meshes[&i.mesh.raw()];
            let transform = i.transform.transpose().to_cols_array()[..12].try_into().unwrap();
            self.bindings.tlas[count] =
                Some(wgpu::TlasInstance::new(m.blas.as_ref().unwrap(), transform, offset, 0xff));
            winding.push(u32::from(i.flip_winding));
            count += 1;
        }
        let _scope = blackbox_trace::scope("Vulkan RT: build TLAS");
        encoder.build_acceleration_structures(std::iter::empty(), [&self.bindings.tlas]);
        blackbox_trace::global().counter(
            "Vulkan RT scene",
            &[
                ("instances", count as f64),
                ("meshes", self.arena.len() as f64),
                ("detail_textures", self.detail.len() as f64),
                ("texture_MiB", (self.atlas.bytes() + self.detail.bytes()) as f64 / 1048576.0),
            ],
        );
        queue.write_buffer(&self.bindings.control, 0, bytemuck::cast_slice(&[count as u32, 0, 0, 0]));
        if !winding.is_empty() {
            queue.write_buffer(&self.bindings.winding, 0, bytemuck::cast_slice(&winding));
        }
        self.live_count = count;
        changed
    }
}
impl Renderer {
    pub(super) fn add_ray_mesh(&mut self, handle: usize, desc: &MeshDesc<'_>) {
        let Some(rt) = &mut self.ray_reflections else { return };
        let layers: Vec<_> = desc
            .draws
            .iter()
            .map(|draw| {
                if matches!(draw.shading, Shading::Sky | Shading::SkyLayer(_)) {
                    return None;
                }
                let texture = match draw.shading {
                    Shading::WorldNormal(h) => self.normal.as_ref().and_then(|n| n.capture_diffuse(h)),
                    _ => draw.texture,
                }
                .map_or(0, |h| h.raw());
                u16::try_from(texture).ok().map(u32::from)
            })
            .collect();
        if let Some(mut mesh) = mesh::Mesh::new(desc, &layers) {
            let surfaces: Vec<_> = desc
                .draws
                .iter()
                .map(|draw| match draw.shading {
                    Shading::WorldNormal(h) => self.normal.as_ref().and_then(|n| n.hit_surface(h)),
                    _ => None,
                })
                .collect();
            surfaces::apply(&mut mesh, desc, &layers, &surfaces);
            rt.meshes.insert(handle, mesh);
            rt.dirty = true;
        }
    }
    pub(super) fn prepare_ray_reflections(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        instances: &[Instance],
        camera: glam::Vec3,
    ) {
        let Some(rt) = &mut self.ray_reflections else { return };
        rt.camera = camera;
        let quality = self.experiments.ray_texture_quality.min(3);
        let resized = quality != rt.quality;
        if resized {
            rt.quality = quality;
            rt.detail = atlas::Atlas::quality(&self.device, quality);
            rt.bindings.detail = rt.detail.view.clone();
            rt.arena = arena::Arena::default();
            rt.dirty = true;
            rt.ranked_at = None;
        }
        let changed = rt.prepare(&self.device, &self.queue, encoder, instances, &self.texture_views) || resized;
        self.queue.write_buffer(
            &rt.bindings.control,
            16,
            bytemuck::cast_slice(&[
                self.experiments.ray_road_blend,
                self.experiments.ray_road_screen_reuse,
                self.experiments.ray_car_blend,
                self.experiments.ray_car_range,
            ]),
        );
        if changed
            && let Some(glossy) = &mut self.glossy
            && let Some(ray) = &mut glossy.ray
        {
            ray.set_scene(&self.device, &rt.bindings, &self.shared.bindings.globals);
        }
        if changed && let Some(normal) = &mut self.normal {
            normal.set_ray_bindings(&self.device, rt.bindings.clone(), &self.shared.bindings.globals);
        }
    }
}
