//! Authored atmospheric sky layers; behavior specified by the game's caller.

mod cloud_map;
mod noise;
mod pipeline;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use blackbox_gfx::{SkyMaterialDesc, SkyMaterialHandle, SkyState};
use wgpu::util::DeviceExt;

use super::{Renderer, meshes::GpuMesh, slots::Slots};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    intensity: [f32; 4],
    options: [f32; 4],
    appearance: [f32; 4],
}

struct Material {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    kind: blackbox_gfx::SkyLayerKind,
    views: [wgpu::TextureView; 2],
}

pub(super) struct Sky {
    shader: wgpu::ShaderModule,
    enhanced_shader: wgpu::ShaderModule,
    cloud_map: cloud_map::CloudMap,
    noise: wgpu::TextureView,
    detail: wgpu::TextureView,
    weather: wgpu::TextureView,
    noise_sampler: wgpu::Sampler,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    materials: Slots<Material>,
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 3]>,
    state: SkyState,
}

impl Sky {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let material_layout = pipeline::material_layout(device);
        let cloud_map = cloud_map::CloudMap::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("authored sky"),
            bind_group_layouts: &[Some(globals), Some(&material_layout), Some(&cloud_map.layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("authored sky shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sky.wgsl").into()),
        });
        let sampler = sampler.clone();
        let enhanced_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("enhanced atmospheric sky shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../../shaders/sky_enhanced.wgsl"),
                    include_str!("../../shaders/sky_clouds.wgsl")
                )
                .into(),
            ),
        });
        let [noise, detail, weather] = noise::create(device, queue);
        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("periodic cloud noise"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            shader,
            enhanced_shader,
            cloud_map,
            noise,
            detail,
            weather,
            noise_sampler,
            layout,
            material_layout,
            sampler,
            materials: Slots::new(),
            pipelines: HashMap::new(),
            state: SkyState::default(),
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        cache: Option<&wgpu::PipelineCache>,
    ) {
        self.pipelines.entry(format).or_insert_with(|| {
            [
                pipeline::create(device, &self.shader, &self.layout, format, false, cache),
                pipeline::create(device, &self.shader, &self.layout, format, true, cache),
                pipeline::create(device, &self.enhanced_shader, &self.layout, format, false, cache),
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
        let scroll = match self.state.enhanced {
            true => scroll * self.state.sky_wind_speed,
            false => scroll,
        };
        for material in self.materials.iter_mut() {
            let coverage = match self.state.cloud_coverage.is_finite() {
                true => self.state.cloud_coverage.clamp(0.0, 1.0),
                false => 0.0,
            };
            let params = Params {
                intensity,
                options: [material.kind as u32 as f32, scroll, 0.0, coverage],
                appearance: [
                    self.state.sky_sun_size,
                    self.state.sky_sun_brightness,
                    self.state.sky_brightness,
                    self.state.sky_sunset_warmth,
                ],
            };
            queue.write_buffer(&material.uniform, 0, bytemuck::bytes_of(&params));
        }
    }

    pub(super) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        format: wgpu::TextureFormat,
        meshes: &'a Slots<GpuMesh>,
        instances: &[blackbox_gfx::Instance],
    ) {
        let Some(pipelines) = self.pipelines.get(&format) else { return };
        pass.set_bind_group(2, &self.cloud_map.group, &[]);
        for kind in
            [blackbox_gfx::SkyLayerKind::Base, blackbox_gfx::SkyLayerKind::Cloud2, blackbox_gfx::SkyLayerKind::Cloud3]
        {
            if self.state.enhanced && kind != blackbox_gfx::SkyLayerKind::Base {
                continue;
            }
            let pipeline = match self.state.enhanced {
                true => 2,
                false => usize::from(kind != blackbox_gfx::SkyLayerKind::Base),
            };
            pass.set_pipeline(&pipelines[pipeline]);
            for (index, instance) in instances.iter().enumerate() {
                let Some(mesh) = meshes.get(instance.mesh.raw()) else { continue };
                for draw in &mesh.draws {
                    let blackbox_gfx::Shading::SkyLayer(handle) = draw.shading else { continue };
                    let Some(material) = self.materials.get(handle.raw()).filter(|m| m.kind == kind) else { continue };
                    pass.set_bind_group(1, &material.bind_group, &[]);
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    let Some(secondary_uvs) = &mesh.secondary_uvs else { continue };
                    pass.set_vertex_buffer(2, secondary_uvs.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        draw.base_vertex,
                        index as u32..index as u32 + 1,
                    );
                }
            }
        }
    }
}

impl Sky {
    pub(super) fn prepare_clouds(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        globals_layout: &wgpu::BindGroupLayout,
        globals_group: &wgpu::BindGroup,
        globals: &super::resources::Globals,
    ) {
        if !self.state.enhanced {
            return;
        }
        let Some(material) = self.materials.iter_mut().find(|m| m.kind == blackbox_gfx::SkyLayerKind::Base) else {
            return;
        };
        self.cloud_map.prepare(
            device,
            encoder,
            &self.enhanced_shader,
            [(globals_layout, globals_group), (&self.material_layout, &material.bind_group)],
            globals,
            self.state,
        );
    }
}

impl Renderer {
    pub fn create_sky_material(&mut self, desc: &SkyMaterialDesc) -> Option<SkyMaterialHandle> {
        let diffuse = self.texture_views.get(&desc.diffuse.raw())?;
        let miscellaneous = self.texture_views.get(&desc.miscellaneous.raw())?;
        let sky = self.sky.get_or_insert_with(|| {
            Sky::new(&self.device, &self.queue, &self.shared.bindings.globals_layout, &self.shared.bindings.sampler)
        });
        let uniform = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky layer parameters"),
            contents: bytemuck::bytes_of(&Params {
                intensity: [1.0; 4],
                options: [desc.kind as u32 as f32, 0.0, 0.0, 0.0],
                appearance: [2.0, 1.0, 1.0, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let views = [diffuse.clone(), miscellaneous.clone()];
        let bind_group = material_group(&self.device, sky, &views, &uniform);
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
            sky.state = state.sanitized();
        }
    }
}

impl Sky {
    pub(super) fn set_sampler(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler) {
        self.sampler = sampler.clone();
        for material in self.materials.iter_mut() {
            material.bind_group = material_group_parts(
                device,
                &self.material_layout,
                sampler,
                [&self.noise, &self.detail, &self.weather],
                &self.noise_sampler,
                &material.views,
                &material.uniform,
            );
        }
    }
}
fn material_group(
    device: &wgpu::Device,
    sky: &Sky,
    views: &[wgpu::TextureView; 2],
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    material_group_parts(
        device,
        &sky.material_layout,
        &sky.sampler,
        [&sky.noise, &sky.detail, &sky.weather],
        &sky.noise_sampler,
        views,
        uniform,
    )
}
fn material_group_parts(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    noise: [&wgpu::TextureView; 3],
    noise_sampler: &wgpu::Sampler,
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
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(noise[0]) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(noise_sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(noise[1]) },
            wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(noise[2]) },
        ],
    })
}
