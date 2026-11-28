//! Optional Vulkan one-bounce reflection scene. See docs/vulkan-ray-reflections.md.
mod arena;
mod atlas;
pub(super) mod bindings;
mod cache;
mod mesh;
mod preparing;
mod ranges;
mod shadow;
mod staging;
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
    /// The textures in the detail cache now, and the ones the last ranking wants there (`staging.rs`).
    detail_textures: std::collections::HashSet<usize>,
    detail_target: std::collections::HashSet<usize>,
    quality: u32,
    camera: glam::Vec3,
    ranked_at: Option<glam::Vec3>,
    pub bindings: Bindings,
    /// Bumped whenever `bindings` change resources, so users can keep bind groups made from them.
    pub generation: u64,
    selected: Vec<usize>,
    arena: arena::Arena,
    offscreen: Vec<Instance>,
    previous: Vec<(usize, [u32; 16], bool, u8)>,
    shadow: Vec<Instance>,
    live_count: usize,
    previous_emission: Vec<(usize, u32)>,
    dirty: bool,
    /// Work carried to the next frame: new meshes still waiting, or detail swaps left (`staging.rs`).
    pending: bool,
    /// Meshes taken out for a detail swap, which come back this frame whatever the budget.
    refresh: std::collections::HashSet<usize>,
    /// New meshes waiting for a later frame. They are left out of the TLAS and mark it incomplete.
    deferred: std::collections::HashSet<usize>,
    /// How much waiting work the next frame does: a frame's budget, or all of it for a one-off capture.
    limits: staging::Limits,
    /// Large meshes whose hit data is being prepared on a worker (`preparing.rs`).
    preparing: preparing::Preparing,
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
            detail_target: Default::default(),
            quality: 1,
            camera: glam::Vec3::ZERO,
            ranked_at: None,
            bindings,
            generation: 0,
            selected: Vec::new(),
            arena: arena::Arena::default(),
            offscreen: Vec::new(),
            previous: Vec::new(),
            shadow: Vec::new(),
            live_count: 0,
            previous_emission: Vec::new(),
            dirty: true,
            pending: false,
            refresh: Default::default(),
            deferred: Default::default(),
            limits: staging::Limits::FRAME,
            preparing: Default::default(),
        })
    }
    pub(in crate::gpu) fn set_material(&self, queue: &wgpu::Queue, handle: usize, packed: Option<&[u8]>) {
        if handle >= bindings::MAX_MATERIALS {
            return;
        }
        let mut bytes = [0u8; bindings::MATERIAL_STRIDE];
        if let Some(packed) = packed {
            bytes.copy_from_slice(packed);
        }
        queue.write_buffer(&self.bindings.materials, (handle * bindings::MATERIAL_STRIDE) as u64, &bytes);
    }
    pub fn remove_mesh(&mut self, handle: usize) {
        self.preparing.cancel(handle);
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
        for (handle, mesh) in self.preparing.finished() {
            let Some(mesh) = mesh else { continue };
            self.meshes.insert(handle, mesh);
            self.dirty = true;
        }
        // Exact mesh/transform identity merges main and offscreen feeds without losing repeated placements.
        let mut unique = BTreeMap::new();
        for (i, mask) in self.offscreen.iter().chain(main).map(|i| (i, 1u8)).chain(self.shadow.iter().map(|i| (i, 2u8)))
        {
            if !self.meshes.contains_key(&i.mesh.raw()) {
                continue;
            }
            if !i.transform.is_finite() || i.transform.determinant().abs() < 1e-12 {
                continue;
            }
            let entry = unique
                .entry((i.mesh.raw(), i.transform.to_cols_array().map(f32::to_bits), i.flip_winding))
                .or_insert((i, 0u8));
            entry.1 |= mask;
        }
        let ids: Vec<_> = unique.keys().map(|k| k.0).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let keys: Vec<_> = unique.iter().map(|(k, (_, mask))| (k.0, k.1, k.2, *mask)).collect();
        if self.dirty
            || self.selected != ids
            || self.ranked_at.is_none_or(|p| p.distance_squared(self.camera) > 32.0 * 32.0)
        {
            let _scope = blackbox_trace::scope("Vulkan RT: rank detail textures");
            let active = ids.iter().flat_map(|id| self.meshes[id].textures.iter().copied()).chain([0]).collect();
            self.atlas.retain(&active);
            self.detail_target = cache::select(
                &self.meshes,
                unique.values().map(|(i, _)| *i),
                views,
                self.camera,
                self.detail.capacity(),
                &self.detail_textures,
            );
            self.ranked_at = Some(self.camera);
        }
        // A few detail swaps a frame: each resamples its texture and re-adds the meshes that use it (`staging.rs`).
        let swaps = staging::detail_step(&self.detail_textures, &self.detail_target, self.limits.swaps);
        for handle in &swaps {
            if !self.detail_textures.remove(handle) {
                self.detail_textures.insert(*handle);
            }
        }
        if !swaps.is_empty() {
            for (id, mesh) in &self.meshes {
                if self.arena.contains(*id) && mesh.textures.iter().any(|h| swaps.contains(h)) {
                    self.arena.remove(*id);
                    self.refresh.insert(*id);
                }
            }
            self.detail.retain(&self.detail_textures);
        }
        // Brightness can change without moving any geometry. Refresh hit data independently of the TLAS.
        let emission: Vec<_> = unique
            .values()
            .enumerate()
            .filter(|(_, (i, _))| i.emission != 0.0)
            .map(|(index, (i, _))| (index, i.emission.to_bits()))
            .collect();
        if !self.dirty && !self.pending && swaps.is_empty() && keys == self.previous {
            if emission != self.previous_emission {
                let words: Vec<_> = unique
                    .values()
                    .filter(|(i, _)| self.arena.offset(i.mesh.raw()).is_some())
                    .take(bindings::MAX_INSTANCES)
                    .map(|(i, _)| {
                        [u32::from(i.flip_winding) | self.meshes[&i.mesh.raw()].cutouts << 1, i.emission.to_bits()]
                    })
                    .collect();
                if !words.is_empty() {
                    queue.write_buffer(&self.bindings.winding, 0, bytemuck::cast_slice(&words));
                }
                self.previous_emission = emission;
            }
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
        self.previous_emission = emission;
        let mut changed = false;
        let capacity = unique.len().max(1).next_power_of_two().min(bindings::MAX_INSTANCES);
        if capacity > self.bindings.tlas.get().len() {
            self.bindings.tlas = bindings::tlas(device, capacity);
            changed = true;
        }
        if self.dirty || self.pending || !swaps.is_empty() || self.selected != ids {
            let _scope = blackbox_trace::scope("Vulkan RT: update hit tables");
            self.arena.retain(&ids);
            let mut new: Vec<_> = ids.iter().copied().filter(|id| !self.arena.contains(*id)).collect();
            let mut distances = HashMap::<usize, f32>::new();
            for (i, _) in unique.values() {
                let distance = i.transform.w_axis.truncate().distance(self.camera);
                distances.entry(i.mesh.raw()).and_modify(|d| *d = d.min(distance)).or_insert(distance);
            }
            staging::nearest_first(&mut new, &distances);
            let mut budget = staging::AddBudget::new(self.limits.add, self.limits.layers);
            let layers_before = self.atlas.len() + self.detail.len();
            self.deferred.clear();
            let mut all_fit = true;
            for id in new {
                let resampled = (self.atlas.len() + self.detail.len()).saturating_sub(layers_before);
                if !self.refresh.contains(&id) && !budget.admit(resampled) {
                    self.deferred.insert(id);
                    continue;
                }
                let tables = (&mut self.atlas, &mut self.detail, &self.detail_textures);
                if !self.arena.ensure(id, &self.meshes[&id], device, encoder, tables, views) {
                    all_fit = false;
                    break;
                }
                let _build = blackbox_trace::scope("Vulkan RT: build BLAS");
                self.meshes.get_mut(&id).expect("selected mesh").build(device, queue);
            }
            self.refresh.clear();
            if !all_fit {
                // Fragmentation can exhaust the high-water mark even when live data fits.
                // Compact only in that exceptional case, retaining the prototype capacity policy.
                self.deferred.clear();
                self.arena = arena::Arena::default();
                for id in &ids {
                    if !self.arena.ensure(
                        *id,
                        &self.meshes[id],
                        device,
                        encoder,
                        (&mut self.atlas, &mut self.detail, &self.detail_textures),
                        views,
                    ) {
                        log::warn!("RT hit-shading buffer limit reached; excess meshes use environment fallback");
                        break;
                    }
                }
            }
            changed |= self.arena.flush(device, queue, &mut self.bindings);
            blackbox_trace::global().counter(
                "Vulkan RT staging",
                &[("deferred_meshes", self.deferred.len() as f64), ("detail_swaps", swaps.len() as f64)],
            );
            self.selected = ids;
            self.dirty = false;
        }
        self.pending = !self.deferred.is_empty() || self.detail_textures != self.detail_target;
        let capacity = self.bindings.tlas.get().len();
        self.bindings.tlas.get_mut_slice(0..capacity).unwrap().fill(None);
        let mut count = 0;
        let mut complete = true;
        let mut winding = Vec::with_capacity(unique.len().min(bindings::MAX_INSTANCES));
        for id in self.arena.ids() {
            self.meshes.get_mut(id).unwrap().build(device, queue);
        }
        for (i, mask) in unique.values() {
            let Some(offset) = self.arena.offset(i.mesh.raw()) else {
                // Any mesh missing from the TLAS, including one still waiting its turn (`staging.rs`), keeps the
                // cascades: a deferred mesh can stay out for many frames, and RT shadows would drop its shadow.
                complete = false;
                continue;
            };
            if count == bindings::MAX_INSTANCES {
                complete = false;
                break;
            }
            let m = &self.meshes[&i.mesh.raw()];
            let transform = i.transform.transpose().to_cols_array()[..12].try_into().unwrap();
            self.bindings.tlas[count] =
                Some(wgpu::TlasInstance::new(m.blas.as_ref().unwrap(), transform, offset, *mask));
            // Bit 0: baked winding flip. Above it: the mesh's cutout count, the start of its opaque geometry.
            winding.push([u32::from(i.flip_winding) | m.cutouts << 1, i.emission.to_bits()]);
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
        queue.write_buffer(&self.bindings.control, 0, bytemuck::cast_slice(&[count as u32, u32::from(complete), 0, 0]));
        if !winding.is_empty() {
            queue.write_buffer(&self.bindings.winding, 0, bytemuck::cast_slice(&winding));
        }
        self.live_count = count;
        changed
    }
}
impl Renderer {
    /// Whether the frames drawn from now on join all waiting meshes and detail swaps at once (`staging.rs`): set
    /// around a one-off capture, which no later frame completes.
    pub(super) fn stage_whole_ray_scene(&mut self, whole: bool) {
        let Some(rt) = &mut self.ray_reflections else { return };
        rt.limits = match whole {
            true => staging::Limits::WHOLE,
            false => staging::Limits::FRAME,
        };
    }
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
        let surfaces: Vec<_> = desc
            .draws
            .iter()
            .map(|draw| match draw.shading {
                Shading::WorldNormal(h) => self.normal.as_ref().and_then(|n| n.hit_surface(h)),
                _ => None,
            })
            .collect();
        rt.preparing.cancel(handle);
        if desc.vertices.len() >= preparing::ASYNC_VERTICES {
            rt.preparing.spawn(handle, desc, layers, surfaces);
            return;
        }
        if let Some(mesh) = preparing::prepare(desc, &layers, &surfaces) {
            rt.meshes.insert(handle, mesh);
            rt.dirty = true;
        }
    }
    pub(super) fn prepare_ray_reflections(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        instances: &[Instance],
        frame: &blackbox_gfx::FrameParams,
    ) {
        let active = self.ray_shadows_active(frame);
        let Some(rt) = &mut self.ray_reflections else { return };
        rt.camera = frame.camera_position;
        rt.select_shadow_casters(self.shadow_casters.as_deref().unwrap_or(instances), frame, &self.experiments, active);
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
        if changed {
            rt.generation = rt.generation.wrapping_add(1);
        }
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
