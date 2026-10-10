//! Glossy shading: the lighting rig, the environment cube map and the registered materials.

mod env;
mod layout;
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
    /// Group 3 per material.
    materials: Slots<wgpu::BindGroup>,
}

impl Glossy {
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
        let view = environment_view(device, queue, SKY_FACE_SIZE, &faces.each_ref().map(Vec::as_slice));
        let scene = scene_group(device, &layouts, &rig, &view, &sampler);
        Self { layouts, rig, sampler, scene, materials: Slots::new() }
    }

    /// Bind the rig and the environment; call after setting a glossy pipeline.
    pub(super) fn bind_scene(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(2, &self.scene, &[]);
    }

    /// Bind one material. Returns `false` (draw nothing) for a destroyed or unknown handle.
    pub(super) fn bind_material(&self, pass: &mut wgpu::RenderPass<'_>, handle: GlossyMaterialHandle) -> bool {
        let Some(group) = self.materials.get(handle.0) else {
            return false;
        };
        pass.set_bind_group(3, group, &[]);
        true
    }
}

fn scene_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    rig: &wgpu::Buffer,
    environment: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glossy scene"),
        layout: &layouts.scene,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: rig.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(environment) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// A cube map view from six RGBA8 faces of `size` × `size` texels (+x, -x, +y, -y, +z, -z).
fn environment_view(device: &wgpu::Device, queue: &wgpu::Queue, size: u32, faces: &[&[u8]; 6]) -> wgpu::TextureView {
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
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    })
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
    /// Whether any glossy resource exists. Nothing glossy (rig, environment cube map, bind groups, pipelines)
    /// is built until the first of [`set_lighting_rig`](Self::set_lighting_rig),
    /// [`set_environment_sky`](Self::set_environment_sky), [`set_environment_faces`](Self::set_environment_faces)
    /// or [`create_glossy_material`](Self::create_glossy_material) is called, so a caller that never shades
    /// glossy pays no memory, shader compile or draw time for it.
    pub fn glossy_in_use(&self) -> bool {
        self.glossy.is_some()
    }

    /// Set the lights every glossy draw is lit by, from the next frame on.
    pub fn set_lighting_rig(&mut self, rig: &LightingRig) {
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        self.queue.write_buffer(&glossy.rig, 0, bytemuck::bytes_of(&RigUniform::new(rig)));
    }

    /// Reflect a generated sky in glossy surfaces (the default).
    pub fn set_environment_sky(&mut self, sky: &SkyGradient) {
        let faces = env::sky_faces(sky, SKY_FACE_SIZE);
        self.set_environment_faces(SKY_FACE_SIZE, faces.each_ref().map(Vec::as_slice));
    }

    /// Reflect a cube map in glossy surfaces: six RGBA8 faces of `size` × `size` texels in the
    /// order +x, -x, +y, -y, +z, -z, addressed by world-space direction.
    pub fn set_environment_faces(&mut self, size: u32, faces: [&[u8]; 6]) {
        let size = size.max(1);
        let texel_bytes = (size * size * 4) as usize;
        if faces.iter().any(|f| f.len() < texel_bytes) {
            log::warn!("environment faces are smaller than {size}x{size} RGBA8: keeping the old environment");
            return;
        }
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        let view = environment_view(&self.device, &self.queue, size, &faces.map(|f| &f[..texel_bytes]));
        glossy.scene = scene_group(&self.device, &glossy.layouts, &glossy.rig, &view, &glossy.sampler);
    }

    /// Register a material for [`Shading::Glossy`](crate::Shading::Glossy) draws.
    pub fn create_glossy_material(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        let glossy = ensure(&mut self.glossy, &self.device, &self.queue, &self.shared.glossy);
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glossy material"),
            contents: bytemuck::bytes_of(&MaterialUniform::new(material)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glossy material"),
            layout: &glossy.layouts.material,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        GlossyMaterialHandle(glossy.materials.insert(group))
    }

    /// Free a material. Draws that still use it are skipped.
    pub fn destroy_glossy_material(&mut self, handle: GlossyMaterialHandle) {
        let Some(glossy) = self.glossy.as_mut() else { return };
        glossy.materials.remove(handle.0);
    }
}
