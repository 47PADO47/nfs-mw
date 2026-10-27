//! Authored atmospheric sky layers; behavior specified by the game's caller.

mod cloud_map;
mod cloud_schedule;
pub(super) mod cloud_shadow;
mod noise;
mod pipeline;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use blackbox_gfx::{SkyMaterialDesc, SkyMaterialHandle, SkyState};
use wgpu::util::DeviceExt;

use super::draw_list::{instance_store, instance_view};
use super::parallel;
use super::{Renderer, draw_list::LayerCmd, meshes::GpuMesh, slots::Slots};

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
    cloud_shadow: cloud_shadow::CloudShadow,
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
    /// The material uniforms are stale: the state changed or a material was added since they were last written.
    params_dirty: bool,
    /// Instances are read from the instance store, bound at group 2; the cloud map follows it.
    store: bool,
}

impl Sky {
    pub(super) fn lighting_state(&self) -> SkyState {
        self.state
    }

    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let material_layout = pipeline::material_layout(device);
        let cloud_map = cloud_map::CloudMap::new(device);
        let cloud_shadow = cloud_shadow::CloudShadow::new(device, globals, &material_layout);
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        // The cloud map follows the instance store's group, when there is one.
        let cloud_group = 2 + u32::from(store);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("authored sky"),
            bind_group_layouts: &match instances.as_ref() {
                Some(instances) => {
                    vec![Some(globals), Some(&material_layout), Some(instances), Some(&cloud_map.layout)]
                }
                None => vec![Some(globals), Some(&material_layout), Some(&cloud_map.layout)],
            },
            immediate_size: 0,
        });
        let source = instance_view::instance_source(include_str!("../../shaders/sky.wgsl"), store.then_some(2));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("authored sky shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler = sampler.clone();
        let enhanced = format!(
            "{}\n{}",
            include_str!("../../shaders/sky_enhanced.wgsl").replace("@group(2)", &format!("@group({cloud_group})")),
            include_str!("../../shaders/sky_clouds.wgsl")
        );
        let enhanced_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("enhanced atmospheric sky shader"),
            source: wgpu::ShaderSource::Wgsl(instance_view::instance_source(&enhanced, store.then_some(2)).into()),
        });
        let [noise, detail, weather] = noise::create(device, queue);
        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("periodic cloud noise"),
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
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
            cloud_shadow,
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
            params_dirty: true,
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
            parallel::build_array([(false, false), (true, false), (false, true)], |&(layer, enhanced)| {
                let shader = match enhanced {
                    true => &self.enhanced_shader,
                    false => &self.shader,
                };
                pipeline::create(device, shader, &self.layout, format, layer, store, cache)
            })
        });
        // Every view of a frame shares the sky state, so the uniforms are rewritten only when it changed.
        if !self.params_dirty {
            return;
        }
        self.params_dirty = false;
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
        // Every pipeline shares the layout, so the cloud map is bound for the authored layers too.
        pass.set_bind_group(2 + u32::from(self.store), &self.cloud_map.group, &[]);
        for kind in
            [blackbox_gfx::SkyLayerKind::Base, blackbox_gfx::SkyLayerKind::Cloud2, blackbox_gfx::SkyLayerKind::Cloud3]
        {
            // The procedural sky replaces every authored layer with one pass, drawn with the base layer's material.
            if self.state.enhanced && kind != blackbox_gfx::SkyLayerKind::Base {
                continue;
            }
            let pipeline = match self.state.enhanced {
                true => 2,
                false => usize::from(kind != blackbox_gfx::SkyLayerKind::Base),
            };
            pass.set_pipeline(&pipelines[pipeline]);
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

impl Sky {
    pub(super) fn cloud_shadow_active(&self) -> bool {
        self.state.enhanced
            && self.state.cloud_coverage > 0.005
            && self.materials.iter_indexed().any(|(_, m)| m.kind == blackbox_gfx::SkyLayerKind::Base)
    }

    pub(super) fn prepare_cloud_shadow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        mut globals: super::resources::Globals,
    ) {
        let Some(material) = self.materials.iter_mut().find(|m| m.kind == blackbox_gfx::SkyLayerKind::Base) else {
            return;
        };
        globals.camera_pos[0] = cloud_shadow::anchor(globals.camera_pos[0]);
        globals.camera_pos[1] = cloud_shadow::anchor(globals.camera_pos[1]);
        globals.camera_pos[2] = 0.0;
        self.cloud_shadow.encode(device, queue, encoder, target, &material.bind_group, &globals);
    }

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
        sky.params_dirty = true;
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

    pub(super) fn glare_clouds(&self) -> Option<&wgpu::TextureView> {
        self.sky.as_ref().filter(|s| s.state.enhanced).map(|s| &s.cloud_map.view)
    }

    pub fn set_sky_state(&mut self, state: SkyState) {
        let state = state.sanitized();
        if self.sky.as_ref().is_none_or(|sky| sky.state == state) {
            return;
        }
        // The next view rewrites the uniforms: a capture queued for the frame keeps the ones it was recorded with.
        self.flush_captures();
        let Some(sky) = &mut self.sky else { return };
        sky.state = state;
        sky.params_dirty = true;
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
