//! Glossy shading: the lighting rig, the environment cube map and the registered materials.

mod env;
mod irradiance;
#[cfg(test)]
mod irradiance_tests;
mod layout;
#[cfg(test)]
mod original_tests;
mod prefilter;
mod probe;
#[cfg(test)]
mod probe_tests;
#[cfg(test)]
mod ray_hit_tests;
mod ray_pass;
#[cfg(test)]
mod ray_tests;
pub(in crate::gpu) mod screen;
pub(in crate::gpu) mod screen_draw;
#[cfg(test)]
mod screen_tests;
#[cfg(test)]
mod surface_tests;
mod uniforms;
#[cfg(test)]
mod validation_tests;

use wgpu::util::DeviceExt;

pub(super) use layout::Layouts;

use super::Renderer;
use super::slots::Slots;
use crate::{GlossyMaterial, GlossyMaterialHandle, LightingRig, SkyGradient};
use uniforms::{MaterialUniform, RigUniform};

/// The WGSL source of the glossy pipelines.
macro_rules! glossy_source {
    ($reflection:literal) => {
        concat!(
            include_str!("../../shaders/glossy.wgsl"),
            "\n",
            include_str!("../../shaders/reflection_screen.wgsl"),
            "\n",
            include_str!($reflection),
            "\n",
            include_str!("../../shaders/reflection_probe.wgsl"),
            include_str!("../../shaders/paint_surface.wgsl"),
            include_str!("../../shaders/material_layers.wgsl")
        )
    };
}
pub(super) const SHADER: &str = glossy_source!("../../shaders/reflection_hybrid.wgsl");
const RAY_SHADER: &str = glossy_source!("../../shaders/reflection_ray_car.wgsl");

/// Side of the generated sky cube's faces.
const SKY_FACE_SIZE: u32 = 64;

pub(super) struct Glossy {
    layouts: Layouts,
    rig: wgpu::Buffer,
    sampler: wgpu::Sampler,
    /// Group 2: the rig and the environment.
    scene: wgpu::BindGroup,
    static_scene: wgpu::BindGroup,
    environment: wgpu::TextureView,
    faces: wgpu::TextureView,
    static_faces: wgpu::TextureView,
    pub(in crate::gpu) screen: screen::Screen,
    static_environment: wgpu::TextureView,
    irradiance: irradiance::Irradiance,
    filtered: Option<prefilter::Prefilter>,
    /// Whether `scene` references a local capture cube, which lives as long as its texture and is drawn into in place.
    capture_bound: bool,
    /// Group 3 per material.
    materials: Slots<Material>,
    /// The vehicle reflection brightness the material blocks were last packed with.
    brightness: f32,
    probe: probe::Probe,
    pub(in crate::gpu) ray: Option<ray_pass::RayPass>,
}

struct Material {
    original: bool,
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// The material as registered, so its block can be packed again when the brightness changes.
    source: GlossyMaterial,
}

impl Glossy {
    pub(super) fn original_material(&self, handle: GlossyMaterialHandle) -> bool {
        self.materials.get(handle.raw()).is_some_and(|m| m.original)
    }

    pub(super) fn has_material(&self, handle: GlossyMaterialHandle) -> bool {
        self.materials.get(handle.raw()).is_some()
    }
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layouts: Layouts,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let rig = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glossy rig"),
            contents: bytemuck::bytes_of(&RigUniform::new(&LightingRig::default())),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glossy environment"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let faces = env::sky_faces(&SkyGradient::default(), SKY_FACE_SIZE);
        let (view, face_view) = environment_view(device, queue, SKY_FACE_SIZE, &faces.each_ref().map(Vec::as_slice));
        let irradiance = irradiance::Irradiance::new(device, cache);
        let screen = screen::Screen::new(device);
        let filtered = prefilter::Prefilter::new(device, cache);
        let probe = probe::Probe::new(device, SKY_FACE_SIZE, faces.each_ref().map(Vec::as_slice));
        let scene = scene_group(
            device,
            &layouts,
            &rig,
            &view,
            &face_view,
            &sampler,
            &irradiance.buffer,
            &screen,
            filtered.as_ref().map_or(&view, |f| &f.view),
            filtered.as_ref().map_or(&view, |f| &f.linear_view),
            &probe,
        );
        Self {
            layouts,
            rig,
            sampler,
            static_scene: scene.clone(),
            scene,
            environment: view.clone(),
            faces: face_view.clone(),
            static_faces: face_view,
            screen,
            static_environment: view,
            irradiance,
            filtered,
            capture_bound: false,
            materials: Slots::new(),
            brightness: 1.0,
            probe,
            ray: None,
        }
    }

    /// Bind the rig and the environment; call after setting a glossy pipeline.
    pub(super) fn use_environment(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        face_view: &wgpu::TextureView,
    ) {
        if self.environment != *view || self.faces != *face_view {
            self.scene = scene_group(
                device,
                &self.layouts,
                &self.rig,
                view,
                face_view,
                &self.sampler,
                &self.irradiance.buffer,
                &self.screen,
                self.filtered.as_ref().map_or(view, |f| &f.view),
                self.filtered.as_ref().map_or(view, |f| &f.linear_view),
                &self.probe,
            );
        }
        self.environment = view.clone();
        self.faces = face_view.clone();
        self.capture_bound = true;
        self.irradiance.dirty = true;
        if let Some(filter) = &mut self.filtered {
            filter.dirty = true;
        }
    }

    pub(super) fn restore_environment(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.probe.control, 0, bytemuck::cast_slice(&[0.0_f32; 4]));
        self.probe.active = false;
        self.scene = self.static_scene.clone();
        self.environment = self.static_environment.clone();
        self.faces = self.static_faces.clone();
        self.capture_bound = false;
        self.irradiance.dirty = true;
        if let Some(filter) = &mut self.filtered {
            filter.dirty = true;
        }
    }

    /// Whether [`use_environment`](Self::use_environment) is still what the scene group holds. A capture binds its cube
    /// once, not on every refresh: the texture is the same, only its contents change.
    pub(super) fn capture_bound(&self) -> bool {
        self.capture_bound
    }

    /// Every face of the bound capture cube is fresh: the diffuse light and the prefiltered copy follow on the next
    /// irradiance update, recorded after the passes that drew it.
    pub(super) fn environment_refreshed(&mut self) {
        self.irradiance.dirty = true;
        if let Some(filter) = &mut self.filtered {
            filter.dirty = true;
        }
    }

    pub(super) fn bind_scene(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(2, &self.scene, &[]);
    }

    /// Bind one material. Returns `false` (draw nothing) for a destroyed or unknown handle.
    pub(super) fn bind_material(&self, pass: &mut wgpu::RenderPass<'_>, handle: GlossyMaterialHandle) -> bool {
        let Some(group) = self.materials.get(handle.raw()) else {
            return false;
        };
        pass.set_bind_group(3, &group.group, &[]);
        true
    }

    /// Packs every material block again when the vehicle brightness changes. A no-op when it has not.
    pub(super) fn sync_brightness(&mut self, queue: &wgpu::Queue, brightness: f32) {
        if self.brightness == brightness {
            return;
        }
        self.brightness = brightness;
        for material in self.materials.iter_mut() {
            let block = MaterialUniform::new(&material.source, brightness);
            queue.write_buffer(&material.buffer, 0, bytemuck::bytes_of(&block));
        }
    }

    pub(super) fn rebind_shadows(&mut self, device: &wgpu::Device, maps: &super::shadows::maps::Maps) {
        for material in self.materials.iter_mut() {
            material.group = maps.material_group(device, &self.layouts.material, &material.buffer);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scene_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    rig: &wgpu::Buffer,
    environment: &wgpu::TextureView,
    face_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    irradiance: &wgpu::Buffer,
    screen: &screen::Screen,
    filtered: &wgpu::TextureView,
    linear_filtered: &wgpu::TextureView,
    probe: &probe::Probe,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glossy scene"),
        layout: &layouts.scene,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: rig.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(environment) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: irradiance.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(face_view) },
            wgpu::BindGroupEntry { binding: 5, resource: screen.control.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&screen.color) },
            wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&screen.depth) },
            wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(filtered) },
            wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(linear_filtered) },
            wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&probe.depth) },
            wgpu::BindGroupEntry { binding: 11, resource: probe.control.as_entire_binding() },
        ],
    })
}

/// A cube map view from six RGBA8 faces of `size` x `size` texels (+x, -x, +y, -y, +z, -z).
fn environment_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
    faces: &[&[u8]; 6],
) -> (wgpu::TextureView, wgpu::TextureView) {
    let extent = wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 6 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glossy environment"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, data) in faces.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
    }
    let cube = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });
    let faces = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    (cube, faces)
}

/// The glossy state, created on first use.
fn ensure<'a>(
    glossy: &'a mut Option<Glossy>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layouts: &Layouts,
    cache: Option<&wgpu::PipelineCache>,
) -> &'a mut Glossy {
    glossy.get_or_insert_with(|| Glossy::new(device, queue, layouts.clone(), cache))
}

impl Renderer {
    /// Build the glossy state now (rig, default environment, irradiance and screen-reflection resources) instead of
    /// at the first call that needs it, so that the first frame with glossy draws does not pay for it.
    pub(in crate::gpu) fn ensure_glossy(&mut self) {
        ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy, self.pipeline_cache.as_ref());
    }

    pub(in crate::gpu) fn supports_irradiance(&self) -> bool {
        let limits = self.device.limits();
        self.info.api != blackbox_gfx::GraphicsApi::Gl
            && limits.max_storage_buffers_per_shader_stage > 0
            && limits.max_storage_buffer_binding_size >= 288
            && limits.max_compute_invocations_per_workgroup >= 9
    }
    pub(in crate::gpu) fn encode_irradiance(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let _trace = blackbox_trace::scope("Renderer CPU: update irradiance");
        let wet = self.experiments.enhanced_wet_surfaces && self.surface_water.dampness > 0.0;
        if wet {
            self.ensure_glossy();
        }
        if let Some(glossy) = &mut self.glossy
            && (wet
                || (self.experiments.paint_response
                    && glossy
                        .materials
                        .iter_mut()
                        .any(|m| m.source.model == blackbox_gfx::GlossyModel::Custom && m.source.surface.is_some())))
            && let Some(filter) = &mut glossy.filtered
        {
            filter.encode(&self.device, encoder, &glossy.environment, &glossy.sampler);
        }
        if !self.experiments.car_skylight || !self.supports_irradiance() {
            return;
        }
        if let Some(glossy) = &mut self.glossy {
            glossy.irradiance.encode(&self.device, encoder, &glossy.environment, &glossy.sampler);
        }
    }
    /// Whether any glossy resource exists. Nothing glossy (rig, environment cube map, bind groups, pipelines)
    /// is built until the first of [`set_lighting_rig`](Self::set_lighting_rig),
    /// [`set_environment_sky`](Self::set_environment_sky), [`set_environment_faces`](Self::set_environment_faces)
    /// or [`create_glossy_material`](Self::create_glossy_material) is called, so a caller that never shades
    /// glossy pays no memory, shader compile or draw time for it.
    pub fn glossy_in_use(&self) -> bool {
        self.glossy.is_some()
    }

    /// Set the lights every glossy draw is lit by, from the next frame on.
    /// The vehicle reflection brightness of the requested settings is applied here too, since the lighting is set
    /// every frame the cars are drawn.
    pub fn set_lighting_rig(&mut self, rig: &LightingRig) {
        // A capture queued for the frame keeps the rig and material blocks it was recorded with.
        self.flush_captures();
        let brightness = self.graphics.vehicle_reflection_brightness.0;
        let glossy =
            ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy, self.pipeline_cache.as_ref());
        let mut block = RigUniform::new(rig);
        block.to_light[0][3] = self.experiments.sun_angle.to_radians().sin().powi(2);
        self.queue.write_buffer(&glossy.rig, 0, bytemuck::bytes_of(&block));
        glossy.sync_brightness(&self.queue, brightness);
    }

    /// Reflect a generated sky in glossy surfaces (the default).
    pub fn set_environment_sky(&mut self, sky: &SkyGradient) {
        let faces = env::sky_faces(sky, SKY_FACE_SIZE);
        self.set_environment_faces(SKY_FACE_SIZE, faces.each_ref().map(Vec::as_slice));
    }

    /// Reflect a cube map in glossy surfaces: six RGBA8 faces of `size` x `size` texels in the
    /// order +x, -x, +y, -y, +z, -z, addressed by world-space direction.
    pub fn set_environment_faces(&mut self, size: u32, faces: [&[u8]; 6]) {
        let size = size.max(1);
        let texel_bytes = (size * size * 4) as usize;
        if faces.iter().any(|f| f.len() < texel_bytes) {
            log::warn!("environment faces are smaller than {size}x{size} RGBA8: keeping the old environment");
            return;
        }
        // A capture queued for the frame keeps the probe settings it was recorded with.
        self.flush_captures();
        let glossy =
            ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy, self.pipeline_cache.as_ref());
        let (view, face_view) = environment_view(&self.device, &self.queue, size, &faces.map(|f| &f[..texel_bytes]));
        self.queue.write_buffer(&glossy.probe.sky, 0, bytemuck::cast_slice(&probe::sky_colours(size, faces)));
        self.queue.write_buffer(&glossy.probe.control, 0, bytemuck::cast_slice(&[0.0_f32; 4]));
        glossy.probe.active = false;
        glossy.scene = scene_group(
            &self.device,
            &glossy.layouts,
            &glossy.rig,
            &view,
            &face_view,
            &glossy.sampler,
            &glossy.irradiance.buffer,
            &glossy.screen,
            glossy.filtered.as_ref().map_or(&view, |f| &f.view),
            glossy.filtered.as_ref().map_or(&view, |f| &f.linear_view),
            &glossy.probe,
        );
        glossy.faces = face_view.clone();
        glossy.static_faces = face_view;
        glossy.environment = view.clone();
        glossy.static_environment = view;
        glossy.capture_bound = false;
        glossy.irradiance.dirty = true;
        if let Some(filter) = &mut glossy.filtered {
            filter.dirty = true;
        }
        glossy.static_scene = glossy.scene.clone();
    }

    /// Register a material for [`Shading::Glossy`](crate::Shading::Glossy) draws.
    pub fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        let brightness = self.graphics.vehicle_reflection_brightness.0;
        let glossy =
            ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy, self.pipeline_cache.as_ref());
        let mut source = *material;
        if glossy.filtered.is_none() {
            source.surface = None;
        }
        let material = &source;
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glossy material"),
            contents: bytemuck::bytes_of(&MaterialUniform::new(material, brightness)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = self.shared.shadows.material_group(&self.device, &glossy.layouts.material, &buffer);
        let handle = GlossyMaterialHandle::from_raw(glossy.materials.insert(Material {
            original: material.model != blackbox_gfx::GlossyModel::Custom,
            source: *material,
            buffer,
            group,
        }));
        if let Some(rt) = &self.ray_reflections {
            rt.set_material(
                &self.queue,
                handle.raw(),
                Some(bytemuck::bytes_of(&MaterialUniform::new(material, brightness).ray_hit())),
            );
        }
        handle
    }

    /// Free a material. Draws that still use it are skipped.
    pub fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        let Some(glossy) = self.glossy.as_mut() else { return };
        glossy.materials.remove(handle.raw());
        if let Some(rt) = &self.ray_reflections {
            rt.set_material(&self.queue, handle.raw(), None);
        }
    }
}
