//! Retained architectural glass; contracts in the world-window material specification.

#[cfg(test)]
mod tests;

use super::{Renderer, meshes::GpuMesh, normal::pipeline, pipelines::BLEND_ORDER, slots::Slots};
use blackbox_gfx::{BlendMode, Shading, WindowMaterialDesc, WindowMaterialHandle};
use std::collections::HashMap;

struct Material {
    group: wgpu::BindGroup,
    views: [wgpu::TextureView; 2],
}

pub(super) struct Window {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    materials: Slots<Material>,
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 4]>,
}

impl Window {
    fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout, shadows: &wgpu::BindGroupLayout) -> Self {
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("architectural glass textures"),
            entries: &[
                texture(0),
                texture(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("architectural glass"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), Some(shadows)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("architectural glass shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../../shaders/window.wgsl"),
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
                )
                .into(),
            ),
        });
        Self { shader, layout, material_layout, materials: Slots::new(), pipelines: HashMap::new() }
    }

    pub(super) fn prepare(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.pipelines.entry(format).or_insert_with(|| {
            BLEND_ORDER.map(|mode| pipeline::create(device, &self.shader, &self.layout, format, mode, false))
        });
    }

    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        mode: BlendMode,
        meshes: &'a Slots<GpuMesh>,
        instances: &[blackbox_gfx::Instance],
        shadows: &'a wgpu::BindGroup,
    ) {
        let Some(pipelines) = self.pipelines.get(&format) else { return };
        let index = BLEND_ORDER.iter().position(|m| *m == mode).expect("known blend mode");
        pass.set_pipeline(&pipelines[index]);
        pass.set_bind_group(2, shadows, &[]);
        let mut first = 0;
        while first < instances.len() {
            let handle = instances[first].mesh;
            let end = first + instances[first..].iter().take_while(|i| i.mesh == handle).count();
            if let Some(mesh) = meshes.get(handle.raw()) {
                for draw in &mesh.draws {
                    if draw.blend != mode {
                        continue;
                    }
                    let Shading::WorldWindow(handle) = draw.shading else { continue };
                    let Some(material) = self.materials.get(handle.raw()) else { continue };
                    pass.set_bind_group(1, &material.group, &[]);
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        draw.base_vertex,
                        first as u32..end as u32,
                    );
                }
            }
            first = end;
        }
    }
}

impl Renderer {
    pub fn create_window_material(&mut self, desc: &WindowMaterialDesc) -> Option<WindowMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let reflection = self.texture_views.get(&desc.reflection.raw())?;
        let window = self.window.get_or_insert_with(|| {
            Window::new(&self.device, &self.shared.bindings.globals_layout, &self.shared.glossy.material)
        });
        let views = [diffuse.clone(), reflection.clone()];
        let group = material_group(&self.device, &window.material_layout, &self.shared.bindings.sampler, &views);
        Some(WindowMaterialHandle::from_raw(window.materials.insert(Material { group, views })))
    }

    pub fn destroy_window_material(&mut self, handle: WindowMaterialHandle) {
        if let Some(window) = &mut self.window {
            window.materials.remove(handle.raw());
        }
    }
}

impl Window {
    pub(super) fn set_sampler(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler) {
        for material in self.materials.iter_mut() {
            material.group = material_group(device, &self.material_layout, sampler, &material.views);
        }
    }
}
fn material_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    views: &[wgpu::TextureView; 2],
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("architectural glass material"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}
