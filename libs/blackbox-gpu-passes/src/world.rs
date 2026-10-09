//! What the world passes (effects, soft particles) share with the scene: the globals uniform, the bind
//! group layouts they are built against, the texture sampler and the target formats.

/// The depth format of the scene and of every depth-tested pass: reverse-Z (1 near, 0 far).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The preferred HDR colour format of an offscreen scene image.
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Which channels the world pipelines write into `format`. The offscreen HDR image keeps the alpha the
/// scene blends; any other target is cleared to alpha 1 and keeps it, so a scene drawn straight into the
/// surface is opaque whatever the compositor's alpha mode.
pub fn write_mask(format: wgpu::TextureFormat) -> wgpu::ColorWrites {
    match format == HDR_FORMAT {
        true => wgpu::ColorWrites::ALL,
        false => wgpu::ColorWrites::COLOR,
    }
}

/// A reverse-Z depth image of `width` x `height` (at least 1x1) that can be rendered to and sampled as a
/// depth texture, which is what the soft-particle pass needs.
pub fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// The `Globals` uniform of the world shaders (bind group 0, binding 0): the camera and the fog.
///
/// The layout is the same in the scene shader and in `effects.wgsl` / `soft_particles.wgsl`, so a renderer
/// that draws the scene itself writes one block per frame and every pass reads it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    /// World to clip space (reverse-Z).
    pub view_proj: [[f32; 4]; 4],
    /// xyz = camera position, w = 1.
    pub camera_pos: [f32; 4],
    /// xyz = unit direction towards the light, w = 0. The effect shaders do not read it.
    pub light_dir: [f32; 4],
    /// rgb = fog colour.
    pub fog_color: [f32; 4],
    /// x = fog start, y = fog end (both in world units; `f32::MAX` turns fog off), z = texture LOD bias.
    pub fog_range: [f32; 4],
}

/// The globals buffer with its bind group, and the layouts and sampler that textured draws use.
///
/// A renderer that owns its own scene pipelines builds one of these and builds those pipelines against
/// [`Self::globals_layout`] and [`Self::texture_layout`]; a renderer that only calls the passes builds one
/// to have a buffer to write [`Globals`] into and a bind group layout for its effect textures.
pub struct WorldBindings {
    /// The [`Globals`] uniform; write it with `Queue::write_buffer` once per frame.
    pub globals: wgpu::Buffer,
    /// Layout of group 0: one uniform buffer, visible to vertex and fragment stages.
    pub globals_layout: wgpu::BindGroupLayout,
    /// Group 0 bound to [`Self::globals`]; set it on the render pass before drawing the world passes.
    pub globals_bind_group: wgpu::BindGroup,
    /// Layout of group 1 for textured draws: a filterable 2D float texture (binding 0) and a sampler (1).
    pub texture_layout: wgpu::BindGroupLayout,
    /// The repeating, anisotropic sampler used for world textures.
    pub sampler: wgpu::Sampler,
}

impl WorldBindings {
    pub fn new(device: &wgpu::Device) -> Self {
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let texture_layout = texture_layout(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("repeat"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        Self { globals, globals_layout, globals_bind_group, texture_layout, sampler }
    }
}

/// The layout of one sampled texture and its sampler (fragment stage).
pub(crate) fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hdr_image_keeps_alpha_and_other_targets_stay_opaque() {
        assert_eq!(write_mask(HDR_FORMAT), wgpu::ColorWrites::ALL);
        assert_eq!(write_mask(wgpu::TextureFormat::Bgra8Unorm), wgpu::ColorWrites::COLOR);
    }

    #[test]
    fn globals_match_the_wgsl_block() {
        // mat4x4 + four vec4: 64 + 4 * 16 bytes.
        assert_eq!(std::mem::size_of::<Globals>(), 128);
    }
}
