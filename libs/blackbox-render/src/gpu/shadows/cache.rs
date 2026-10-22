//! Conservative static depth reuse. Exact projections and resource identities define validity.
use crate::gpu::Renderer;
use blackbox_gfx::{Instance, InstanceKey};
use glam::Mat4;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub(super) struct Cache {
    previous: HashMap<InstanceKey, Stamp>,
    pub stable: Vec<bool>,
    projections: [Option<[u32; 16]>; 4],
    entries: [Option<Entry>; 4],
}
#[derive(PartialEq, Eq, Clone, Copy)]
struct Stamp {
    mesh: usize,
    transform: [u32; 16],
}
struct Entry {
    image: wgpu::TextureView,
    fingerprint: u64,
    final_target: Option<wgpu::TextureView>,
    moving: bool,
}
impl Cache {
    pub fn prepare(&mut self, casters: &[Instance]) {
        let mut current = HashMap::with_capacity(casters.len());
        let mut counts = HashMap::with_capacity(casters.len());
        for instance in casters {
            *counts.entry(instance.key).or_insert(0u32) += 1;
        }
        self.stable.clear();
        self.stable.extend(casters.iter().map(|instance| {
            if instance.key.is_transient() || counts[&instance.key] != 1 {
                return false;
            }
            let stamp =
                Stamp { mesh: instance.mesh.raw(), transform: instance.transform.to_cols_array().map(f32::to_bits) };
            current.insert(instance.key, stamp);
            self.previous.get(&instance.key) == Some(&stamp)
        }));
        self.previous = current;
    }
    pub fn projection(&mut self, cascade: usize, matrix: Mat4, cut: bool) -> bool {
        let bits = matrix.to_cols_array().map(f32::to_bits);
        let same = !cut && self.projections[cascade] == Some(bits);
        self.projections[cascade] = Some(bits);
        same
    }
    pub fn invalidate_final(&mut self, cascade: usize) {
        if let Some(entry) = &mut self.entries[cascade] {
            entry.final_target = None;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &mut self,
        device: &wgpu::Device,
        cascade: usize,
        size: u32,
        fingerprint: u64,
        target: &wgpu::TextureView,
        moving: bool,
    ) -> (wgpu::TextureView, bool, bool) {
        let entry = self.entries[cascade].get_or_insert_with(|| Entry {
            image: image(device, size),
            fingerprint: fingerprint.wrapping_add(1),
            final_target: None,
            moving: true,
        });
        let mut dirty = entry.fingerprint != fingerprint;
        if entry.image.texture().width() != size {
            entry.image = image(device, size);
            dirty = true;
        }
        let reuse_final = !dirty && !moving && !entry.moving && entry.final_target.as_ref() == Some(target);
        entry.fingerprint = fingerprint;
        entry.final_target = Some(target.clone());
        entry.moving = moving;
        (entry.image.clone(), dirty, reuse_final)
    }
    pub fn mib(&self) -> u32 {
        self.entries.iter().flatten().map(|entry| entry.image.texture().width().pow(2) * 4 / (1024 * 1024)).sum()
    }
}
fn image(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("cached static sun depth"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
pub(super) fn fingerprint(renderer: &Renderer, matrix: Mat4, selected: &[usize], casters: &[Instance]) -> u64 {
    let mut hash = DefaultHasher::new();
    matrix.to_cols_array().map(f32::to_bits).hash(&mut hash);
    renderer.experiments.original_world_materials.hash(&mut hash);
    for &index in selected {
        let instance = &casters[index];
        instance.key.hash(&mut hash);
        instance.transform.to_cols_array().map(f32::to_bits).hash(&mut hash);
        let Some(mesh) = renderer.meshes.get(instance.mesh.raw()) else { continue };
        mesh.vertices.hash(&mut hash);
        mesh.indices.hash(&mut hash);
        for draw in &mesh.draws {
            // Immutable mesh draw metadata belongs to these vertex/index resource identities.
            let slot = draw.texture.map(|t| renderer.redirects.resolve(t.raw()));
            slot.and_then(|s| renderer.textures.get(s)).or(renderer.textures.get(0)).hash(&mut hash);
            if let blackbox_gfx::Shading::WorldNormal(handle) = draw.shading {
                renderer.normal.as_ref().is_some_and(|n| n.reflective_material(handle)).hash(&mut hash);
            }
        }
    }
    hash.finish()
}
