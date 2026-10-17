//! Glossy shading: the lighting rig, the environment cube map and the registered materials.

mod env;
mod irradiance;
#[cfg(test)]
mod irradiance_tests;
mod layout;
#[cfg(test)]
mod original_tests;
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
pub(super) const SHADER: &str = include_str!("../../shaders/glossy.wgsl");

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
    static_environment: wgpu::TextureView,
    irradiance: irradiance::Irradiance,
    /// Group 3 per material.
    materials: Slots<Material>,
    /// The vehicle reflection brightness the material blocks were last packed with.
    brightness: f32,
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
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, layouts: Layouts) -> Self {
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
            ..Default::default()
        });
        let faces = env::sky_faces(&SkyGradient::default(), SKY_FACE_SIZE);
        let (view, face_view) = environment_view(device, queue, SKY_FACE_SIZE, &faces.each_ref().map(Vec::as_slice));
        let irradiance = irradiance::Irradiance::new(device);
        let scene = scene_group(device, &layouts, &rig, &view, &face_view, &sampler, &irradiance.buffer);
        Self {
            layouts,
            rig,
            sampler,
            static_scene: scene.clone(),
            scene,
            environment: view.clone(),
            static_environment: view,
            irradiance,
            materials: Slots::new(),
            brightness: 1.0,
        }
    }

    /// Bind the rig and the environment; call after setting a glossy pipeline.
    pub(super) fn use_environment(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        face_view: &wgpu::TextureView,
    ) {
        self.scene =
            scene_group(device, &self.layouts, &self.rig, view, face_view, &self.sampler, &self.irradiance.buffer);
        self.environment = view.clone();
        self.irradiance.dirty = true;
    }

    pub(super) fn restore_environment(&mut self) {
        self.scene = self.static_scene.clone();
        self.environment = self.static_environment.clone();
        self.irradiance.dirty = true;
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

fn scene_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    rig: &wgpu::Buffer,
    environment: &wgpu::TextureView,
    face_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    irradiance: &wgpu::Buffer,
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
) -> &'a mut Glossy {
    glossy.get_or_insert_with(|| Glossy::new(device, queue, layouts.clone()))
}

impl Renderer {
    pub(in crate::gpu) fn supports_irradiance(&self) -> bool {
        let limits = self.device.limits();
        self.info.api != blackbox_gfx::GraphicsApi::Gl
            && limits.max_storage_buffers_per_shader_stage > 0
            && limits.max_storage_buffer_binding_size >= 288
            && limits.max_compute_invocations_per_workgroup >= 9
    }
    pub(in crate::gpu) fn update_irradiance(&mut self) {
        if !self.experiments.car_skylight || !self.supports_irradiance() {
            return;
        }
        if let Some(glossy) = &mut self.glossy {
            glossy.irradiance.update(&self.device, &self.queue, &glossy.environment, &glossy.sampler);
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
        let brightness = self.graphics.vehicle_reflection_brightness.0;
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        self.queue.write_buffer(&glossy.rig, 0, bytemuck::bytes_of(&RigUniform::new(rig)));
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
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        let (view, face_view) = environment_view(&self.device, &self.queue, size, &faces.map(|f| &f[..texel_bytes]));
        glossy.scene = scene_group(
            &self.device,
            &glossy.layouts,
            &glossy.rig,
            &view,
            &face_view,
            &glossy.sampler,
            &glossy.irradiance.buffer,
        );
        glossy.environment = view.clone();
        glossy.static_environment = view;
        glossy.irradiance.dirty = true;
        glossy.static_scene = glossy.scene.clone();
    }

    /// Register a material for [`Shading::Glossy`](crate::Shading::Glossy) draws.
    pub fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        let brightness = self.graphics.vehicle_reflection_brightness.0;
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glossy material"),
            contents: bytemuck::bytes_of(&MaterialUniform::new(material, brightness)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = self.shared.shadows.material_group(&self.device, &glossy.layouts.material, &buffer);
        GlossyMaterialHandle::from_raw(glossy.materials.insert(Material {
            original: material.model != blackbox_gfx::GlossyModel::Custom,
            source: *material,
            buffer,
            group,
        }))
    }

    /// Free a material. Draws that still use it are skipped.
    pub fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        let Some(glossy) = self.glossy.as_mut() else { return };
        glossy.materials.remove(handle.raw());
    }
}
