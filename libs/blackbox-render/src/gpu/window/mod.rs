//! Retained architectural glass; contracts in the world-window material specification.

#[cfg(test)]
mod tests;

use super::draw_list::{instance_store, instance_view};
use super::{
    Renderer, cull::Face, draw_list::LayerCmd, meshes::GpuMesh, normal::pipeline, pipelines::BLEND_ORDER, slots::Slots,
};
use blackbox_gfx::{BlendMode, Shading, WindowMaterialDesc, WindowMaterialHandle};
use std::collections::HashMap;

struct Material {
    group: wgpu::BindGroup,
    scene_group: Option<wgpu::BindGroup>,
    views: [wgpu::TextureView; 2],
}

pub(super) struct Window {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    fallback: wgpu::TextureView,
    environment: Option<wgpu::TextureView>,
    materials: Slots<Material>,
    /// Per target format and whether the variant discards fragments behind the clip plane (`(format, clip)`).
    pipelines: HashMap<(wgpu::TextureFormat, bool), pipeline::FaceSets>,
    /// Pick and build the clip-plane variants (a planar-reflection capture); the main view leaves it off.
    clip: bool,
    /// Instances are read from the instance store, bound at group 3.
    store: bool,
}

impl Window {
    fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout, shadows: &wgpu::BindGroupLayout) -> Self {
        let texture = |binding, view_dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension,
                multisampled: false,
            },
            count: None,
        };
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("architectural glass textures"),
            entries: &[
                texture(0, wgpu::TextureViewDimension::D2),
                texture(1, wgpu::TextureViewDimension::D2),
                texture(3, wgpu::TextureViewDimension::Cube),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("architectural glass"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), Some(shadows), instances.as_ref()]
                [..3 + usize::from(store)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("architectural glass shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    instance_view::instance_source(include_str!("../../shaders/window.wgsl"), store.then_some(3)),
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
                )
                .into(),
            ),
        });
        let fallback = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("window capture fallback"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
        Self {
            shader,
            layout,
            material_layout,
            fallback,
            environment: None,
            materials: Slots::new(),
            pipelines: HashMap::new(),
            clip: false,
            store,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        (format, culled): (wgpu::TextureFormat, bool),
        cache: Option<&wgpu::PipelineCache>,
    ) {
        let (store, shader, layout, clip) = (self.store, &self.shader, &self.layout, self.clip);
        pipeline::fill_faces(self.pipelines.entry((format, clip)).or_default(), culled, |mode, face| {
            pipeline::create(device, shader, layout, format, (mode, face), false, store, clip, cache)
        });
    }

    pub(super) fn set_environment(
        &mut self,
        device: &wgpu::Device,
        sampler: &wgpu::Sampler,
        view: Option<&wgpu::TextureView>,
    ) {
        if self.environment.as_ref() == view {
            return;
        }
        self.environment = view.cloned();
        for material in self.materials.iter_mut() {
            material.scene_group =
                view.map(|v| material_group(device, &self.material_layout, sampler, &material.views, v));
        }
    }

    /// Pick (and from the next [`Self::prepare`] on, build) the variants that discard fragments behind the clip plane.
    pub(super) fn use_clip(&mut self, clip: bool) {
        self.clip = clip;
    }

    /// Draw the architectural glass of blend mode `mode`: `list` holds only those draws, in the order they were
    /// given. The material and the mesh's buffers are bound only when they change. `instances` is the instance
    /// store's bind group, when the renderer has one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        (mode, mirrored_view, capture): (BlendMode, bool, bool),
        meshes: &'a Slots<GpuMesh>,
        list: &[LayerCmd],
        shadows: &'a wgpu::BindGroup,
        instances: Option<&wgpu::BindGroup>,
    ) {
        if list.is_empty() {
            return;
        }
        let Some(pipelines) = self.pipelines.get(&(format, self.clip)) else { return };
        let index = BLEND_ORDER.iter().position(|m| *m == mode).expect("known blend mode");
        let Some(two_sided) = pipeline::pick(pipelines, index, Face::Both) else { return };
        pass.set_pipeline(two_sided);
        let mut bound_face = Face::Both;
        pass.set_bind_group(2, shadows, &[]);
        if let Some(instances) = instances.filter(|_| self.store) {
            pass.set_bind_group(3, instances, &[]);
        }
        let mut bound_mesh = None;
        let mut bound_material = None;
        for cmd in list {
            let Some(mesh) = meshes.get(cmd.mesh) else { continue };
            let Some(draw) = mesh.draws.get(cmd.draw as usize) else { continue };
            let Shading::WorldWindow(handle) = draw.shading else { continue };
            let Some(material) = self.materials.get(handle.raw()) else { continue };
            if bound_material != Some(handle.raw()) {
                let group = material.scene_group.as_ref().filter(|_| !capture).unwrap_or(&material.group);
                pass.set_bind_group(1, group, &[]);
                bound_material = Some(handle.raw());
            }
            let face = cmd.face.seen(mirrored_view);
            if face != bound_face
                && let Some(p) = pipeline::pick(pipelines, index, face)
            {
                pass.set_pipeline(p);
                bound_face = face;
            }
            if bound_mesh != Some(cmd.mesh) {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
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

impl Renderer {
    pub fn create_window_material(&mut self, desc: &WindowMaterialDesc) -> Option<WindowMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let reflection = self.texture_views.get(&desc.reflection.raw())?;
        let window = self.window.get_or_insert_with(|| {
            Window::new(&self.device, &self.shared.bindings.globals_layout, &self.shared.glossy.material)
        });
        let views = [diffuse.clone(), reflection.clone()];
        let group = material_group(
            &self.device,
            &window.material_layout,
            &self.shared.bindings.sampler,
            &views,
            &window.fallback,
        );
        let scene_group = window
            .environment
            .as_ref()
            .map(|v| material_group(&self.device, &window.material_layout, &self.shared.bindings.sampler, &views, v));
        Some(WindowMaterialHandle::from_raw(window.materials.insert(Material { group, scene_group, views })))
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
            material.group = material_group(device, &self.material_layout, sampler, &material.views, &self.fallback);
            material.scene_group = self
                .environment
                .as_ref()
                .map(|v| material_group(device, &self.material_layout, sampler, &material.views, v));
        }
    }
}
fn material_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    views: &[wgpu::TextureView; 2],
    environment: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("architectural glass material"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(environment) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}
