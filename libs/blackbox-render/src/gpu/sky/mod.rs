//! Authored atmospheric sky layers; behavior specified by the game's caller.

mod pipeline;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use blackbox_gfx::{SkyMaterialDesc, SkyMaterialHandle, SkyState};
use wgpu::util::DeviceExt;

use super::draw_list::{instance_store, instance_view};
use super::{Renderer, draw_list::LayerCmd, meshes::GpuMesh, slots::Slots};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    intensity: [f32; 4],
    options: [f32; 4],
}

struct Material {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    kind: blackbox_gfx::SkyLayerKind,
    views: [wgpu::TextureView; 2],
}

pub(super) struct Sky {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    materials: Slots<Material>,
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 2]>,
    state: SkyState,
    /// Instances are read from the instance store, bound at group 2.
    store: bool,
}

impl Sky {
    fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler) -> Self {
        let material_layout = pipeline::material_layout(device);
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("authored sky"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), instances.as_ref()][..2 + usize::from(store)],
            immediate_size: 0,
        });
        let source = instance_view::instance_source(include_str!("../../shaders/sky.wgsl"), store.then_some(2));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("authored sky shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler = sampler.clone();
        Self {
            shader,
            layout,
            material_layout,
            sampler,
            materials: Slots::new(),
            pipelines: HashMap::new(),
            state: SkyState::default(),
            store,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        cache: Option<&wgpu::PipelineCache>,
    ) {
        let store = self.store;
        self.pipelines.entry(format).or_insert_with(|| {
            [
                pipeline::create(device, &self.shader, &self.layout, format, false, store, cache),
                pipeline::create(device, &self.shader, &self.layout, format, true, store, cache),
            ]
        });
        let intensity = self.state.cloud_intensity.map(|x| match x.is_finite() {
            true => x.clamp(-1.0, 1.0),
            false => 0.0,
        });
        let scroll = match self.state.scroll.is_finite() {
            true => self.state.scroll,
            false => 0.0,
        };
        for material in self.materials.iter_mut() {
            let params = Params { intensity, options: [material.kind as u32 as f32, scroll, 0.0, 0.0] };
            queue.write_buffer(&material.uniform, 0, bytemuck::bytes_of(&params));
        }
    }

    /// Draw the sky layers: each layer kind in turn, over every sky draw of the view in instance order. `list`
    /// holds only the sky draws, so a view with no sky costs nothing here. State is bound only when it changes.
    /// `instances` is the instance store's bind group, when the renderer has one.
    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        meshes: &'a Slots<GpuMesh>,
        list: &[LayerCmd],
        instances: Option<&wgpu::BindGroup>,
    ) {
        if list.is_empty() {
            return;
        }
        let Some(pipelines) = self.pipelines.get(&format) else { return };
        if let Some(instances) = instances.filter(|_| self.store) {
            pass.set_bind_group(2, instances, &[]);
        }
        for kind in
            [blackbox_gfx::SkyLayerKind::Base, blackbox_gfx::SkyLayerKind::Cloud2, blackbox_gfx::SkyLayerKind::Cloud3]
        {
            pass.set_pipeline(&pipelines[usize::from(kind != blackbox_gfx::SkyLayerKind::Base)]);
            let mut bound_mesh = None;
            let mut bound_material = None;
            for cmd in list {
                let Some(mesh) = meshes.get(cmd.mesh) else { continue };
                let Some(draw) = mesh.draws.get(cmd.draw as usize) else { continue };
                let blackbox_gfx::Shading::SkyLayer(handle) = draw.shading else { continue };
                let Some(material) = self.materials.get(handle.raw()).filter(|m| m.kind == kind) else { continue };
                let Some(secondary_uvs) = &mesh.secondary_uvs else { continue };
                if bound_material != Some(handle.raw()) {
                    pass.set_bind_group(1, &material.bind_group, &[]);
                    bound_material = Some(handle.raw());
                }
                if bound_mesh != Some(cmd.mesh) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_vertex_buffer(2, secondary_uvs.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    bound_mesh = Some(cmd.mesh);
                }
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    draw.base_vertex,
                    cmd.instances.0..cmd.instances.1,
                );
            }
        }
    }
}

impl Renderer {
    pub fn create_sky_material(&mut self, desc: &SkyMaterialDesc) -> Option<SkyMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let miscellaneous = self.texture_views.get(&desc.miscellaneous.raw())?;
        let sky = self.sky.get_or_insert_with(|| {
            Sky::new(&self.device, &self.shared.bindings.globals_layout, &self.shared.bindings.sampler)
        });
        let uniform = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky layer parameters"),
            contents: bytemuck::bytes_of(&Params {
                intensity: [1.0; 4],
                options: [desc.kind as u32 as f32, 0.0, 0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let views = [diffuse.clone(), miscellaneous.clone()];
        let bind_group = material_group(&self.device, &sky.material_layout, &sky.sampler, &views, &uniform);
        Some(SkyMaterialHandle::from_raw(sky.materials.insert(Material {
            uniform,
            bind_group,
            kind: desc.kind,
            views,
        })))
    }

    pub fn destroy_sky_material(&mut self, handle: SkyMaterialHandle) {
        if let Some(sky) = &mut self.sky {
            sky.materials.remove(handle.raw());
        }
    }

    pub fn set_sky_state(&mut self, state: SkyState) {
        if let Some(sky) = &mut self.sky {
            sky.state = state;
        }
    }
}

impl Sky {
    pub(super) fn set_sampler(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler) {
        self.sampler = sampler.clone();
        for material in self.materials.iter_mut() {
            material.bind_group =
                material_group(device, &self.material_layout, sampler, &material.views, &material.uniform);
        }
    }
}
fn material_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    views: &[wgpu::TextureView; 2],
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sky layer material"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: uniform.as_entire_binding() },
        ],
    })
}
