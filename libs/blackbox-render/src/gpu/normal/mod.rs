//! Retained tangent-space scenery materials; per-frame light data is shared.

pub(super) mod pipeline;
#[cfg(test)]
mod reflect_tests;
#[cfg(test)]
mod surface_boundary_tests;
#[cfg(test)]
mod tests;

use super::{Renderer, draw_list::LayerCmd, meshes::GpuMesh, pipelines::BLEND_ORDER, slots::Slots};
use blackbox_gfx::{BlendMode, FrameParams, NormalMaterialDesc, NormalMaterialHandle};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

struct Material {
    bind_group: wgpu::BindGroup,
    model: blackbox_gfx::NormalModel,
    views: [wgpu::TextureView; 4],
    offset: wgpu::Buffer,
    uses_projected: bool,
    diffuse: blackbox_gfx::TextureHandle,
}

pub(super) struct Normal {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    lighting: wgpu::Buffer,
    black_reflection: wgpu::TextureView,
    projected: Option<wgpu::TextureView>,
    materials: Slots<Material>,
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 4]>,
}

impl Normal {
    fn new(
        device: &wgpu::Device,
        globals: &wgpu::BindGroupLayout,
        shadows: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let material_layout = pipeline::material_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("normal scenery"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), Some(shadows)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("normal scenery shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../../shaders/normal.wgsl"),
                    include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
                )
                .into(),
            ),
        });
        let sampler = sampler.clone();
        let lighting = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("normal scenery shared specular"),
            contents: bytemuck::cast_slice(&[0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // WebGPU initializes new textures to zero. Absence of a projected view adds no light.
        let black_reflection = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("absent projected reflection"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Self {
            shader,
            layout,
            material_layout,
            sampler,
            lighting,
            black_reflection,
            projected: None,
            materials: Slots::new(),
            pipelines: HashMap::new(),
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        frame: &FrameParams,
        cache: Option<&wgpu::PipelineCache>,
    ) {
        self.pipelines.entry(format).or_insert_with(|| {
            BLEND_ORDER.map(|mode| pipeline::create(device, &self.shader, &self.layout, format, mode, true, cache))
        });
        let specular = frame
            .surface_light
            .map_or([0.0, 0.0, 0.0, 1.0], |l| [l.specular[0], l.specular[1], l.specular[2], l.specular_power]);
        let reflection = frame.surface_light.map_or(0.0, |l| l.reflection_scale);
        let highlight_scale = frame.surface_light.map_or(1.0, |l| l.reflective_specular_scale);
        queue.write_buffer(
            &self.lighting,
            0,
            bytemuck::cast_slice(&[specular, [reflection, highlight_scale, 0.0, 0.0]]),
        );
    }

    /// Draw the normal-map scenery of blend mode `mode`: `list` holds only those draws, in the order they were
    /// given. The material and the mesh's buffers are bound only when they change.
    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        mode: BlendMode,
        meshes: &'a Slots<GpuMesh>,
        list: &[LayerCmd],
        shadows: &'a wgpu::BindGroup,
    ) {
        if list.is_empty() {
            return;
        }
        let Some(pipelines) = self.pipelines.get(&format) else { return };
        let index = BLEND_ORDER.iter().position(|m| *m == mode).expect("known blend mode");
        pass.set_pipeline(&pipelines[index]);
        pass.set_bind_group(2, shadows, &[]);
        let mut bound_mesh = None;
        let mut bound_material = None;
        for cmd in list {
            let Some(mesh) = meshes.get(cmd.mesh) else { continue };
            let Some(draw) = mesh.draws.get(cmd.draw as usize) else { continue };
            let blackbox_gfx::Shading::WorldNormal(handle) = draw.shading else { continue };
            let Some(material) = self.materials.get(handle.raw()) else { continue };
            let Some(tangents) = &mesh.tangents else { continue };
            if bound_material != Some(handle.raw()) {
                pass.set_bind_group(1, &material.bind_group, &[]);
                bound_material = Some(handle.raw());
            }
            if bound_mesh != Some(cmd.mesh) {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(2, tangents.slice(..));
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

    pub(super) fn reflective_material(&self, handle: NormalMaterialHandle) -> bool {
        self.materials.get(handle.raw()).is_some_and(|m| m.model == blackbox_gfx::NormalModel::Reflective)
    }
}

impl Renderer {
    pub(super) fn reflective_surface(&self, shading: blackbox_gfx::Shading) -> bool {
        if !self.experiments.original_world_materials {
            return false;
        }
        let blackbox_gfx::Shading::WorldNormal(handle) = shading else { return false };
        self.normal.as_ref().is_some_and(|normal| normal.reflective_material(handle))
    }

    pub fn create_normal_material(&mut self, desc: &NormalMaterialDesc) -> Option<NormalMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let normal_map = self.texture_views.get(&desc.normal.raw())?;
        let specular = self.texture_views.get(&desc.specular.raw())?;
        let normal = self.normal.get_or_insert_with(|| {
            Normal::new(
                &self.device,
                &self.shared.bindings.globals_layout,
                &self.shared.glossy.material,
                &self.shared.bindings.sampler,
            )
        });
        normal.set_projected(&self.device, self.projected.as_ref().map(|p| p.output()));
        let reflection = match desc.reflection {
            Some(texture) => self.texture_views.get(&texture.raw())?,
            None if desc.model == blackbox_gfx::NormalModel::Reflective => {
                normal.projected.as_ref().unwrap_or(&normal.black_reflection)
            }
            None => &normal.black_reflection,
        };
        let offset = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("normal scenery UV offset"),
            contents: bytemuck::cast_slice(&[
                desc.uv_offset[0],
                desc.uv_offset[1],
                u8::from(desc.model == blackbox_gfx::NormalModel::Reflective) as f32,
                0.0,
            ]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let views = [diffuse.clone(), normal_map.clone(), specular.clone(), reflection.clone()];
        let bind_group =
            material_group(&self.device, &normal.material_layout, &normal.sampler, &normal.lighting, &views, &offset);
        Some(NormalMaterialHandle::from_raw(normal.materials.insert(Material {
            bind_group,
            model: desc.model,
            uses_projected: desc.reflection.is_none() && desc.model == blackbox_gfx::NormalModel::Reflective,
            diffuse: desc.diffuse,
            views,
            offset,
        })))
    }

    pub fn destroy_normal_material(&mut self, handle: NormalMaterialHandle) {
        if let Some(normal) = &mut self.normal {
            normal.materials.remove(handle.raw());
        }
    }
}

impl Normal {
    pub(super) fn set_sampler(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler) {
        self.sampler = sampler.clone();
        for material in self.materials.iter_mut() {
            material.bind_group = material_group(
                device,
                &self.material_layout,
                sampler,
                &self.lighting,
                &material.views,
                &material.offset,
            );
        }
    }
}
fn material_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    lighting: &wgpu::Buffer,
    views: &[wgpu::TextureView; 4],
    offset: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("normal scenery material"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 4, resource: offset.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: lighting.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&views[3]) },
        ],
    })
}

impl Normal {
    pub(super) fn set_projected(&mut self, device: &wgpu::Device, view: Option<&wgpu::TextureView>) {
        let same = match (&self.projected, view) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.projected = view.cloned();
        let view = self.projected.as_ref().unwrap_or(&self.black_reflection);
        for material in self.materials.iter_mut().filter(|m| m.uses_projected) {
            material.views[3] = view.clone();
            material.bind_group = material_group(
                device,
                &self.material_layout,
                &self.sampler,
                &self.lighting,
                &material.views,
                &material.offset,
            );
        }
    }
    pub(super) fn capture_diffuse(&self, handle: NormalMaterialHandle) -> Option<blackbox_gfx::TextureHandle> {
        self.materials.get(handle.raw()).map(|m| m.diffuse)
    }
}
