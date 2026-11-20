//! Colour grading through 3D look-up tables: one fullscreen pass that reads the finished frame, looks each pixel up in a
//! cube (one linearly filtered 3D texture sample, two when a second cube is blended in) and moves the colour a share of the
//! way to the result. Engine-generic: the caller gives a [`blackbox_gfx::ColourGrade`] and the images. Spec:
//! `docs/specs/colour-grading.md`.
//!
//! The pass keeps the cubes it has uploaded (RGBA8 3D textures, by [`ColourCube::id`]) and frees the ones not used for a
//! while, so a grade that arrives again every frame uploads nothing.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_gfx::{ColourCube, ColourGrade};

/// How many cubes stay uploaded at most (a 32 cube is 128 KiB, a 64 cube 1 MiB).
const RESIDENT_CUBES: usize = 16;

const SHADER: &str = include_str!("shaders/colour_cube.wgsl");

/// The shader's source (the tests validate it without a GPU).
pub const COLOUR_CUBE_WGSL: &str = SHADER;

/// The images of one draw.
pub struct ColourCubeIo<'a> {
    pub input: &'a wgpu::TextureView,
    pub output: &'a wgpu::TextureView,
    pub output_format: wgpu::TextureFormat,
}

struct Resident {
    view: wgpu::TextureView,
    last_used: u64,
}

/// What the pass draws: the cubes' ids and the numbers of the shader.
struct Active {
    from: u64,
    to: Option<u64>,
    mix: f32,
    strength: f32,
}

/// The parameter block of the shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Block {
    a: [f32; 4],
}

/// The colour cube pass.
pub struct ColourCubePass {
    shader: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    cubes: HashMap<u64, Resident>,
    tick: u64,
    active: Option<Active>,
    cache: Option<wgpu::PipelineCache>,
}

fn texture_entry(binding: u32, dimension: wgpu::TextureViewDimension) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    }
}

impl ColourCubePass {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("colour cube"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        use wgpu::TextureViewDimension::{D2, D3};
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("colour cube"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D3),
                texture_entry(2, D3),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("colour cube"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let clamp = wgpu::AddressMode::ClampToEdge;
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("colour cube"),
            address_mode_u: clamp,
            address_mode_v: clamp,
            address_mode_w: clamp,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("colour cube"),
            size: std::mem::size_of::<Block>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            shader,
            layout,
            pipeline_layout,
            sampler,
            params,
            pipelines: HashMap::new(),
            cubes: HashMap::new(),
            tick: 0,
            active: None,
            cache: None,
        }
    }

    /// Build the pipelines with `cache` (a `wgpu::PipelineCache`); without one they compile from scratch.
    pub fn with_pipeline_cache(mut self, cache: Option<wgpu::PipelineCache>) -> Self {
        self.cache = cache;
        self
    }

    /// Whether a grade is set that changes something: the pass has nothing to draw otherwise.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// How many cubes are uploaded.
    pub fn resident(&self) -> usize {
        self.cubes.len()
    }

    /// Set the grade for the coming draws (`None` or a neutral grade stops them). Uploads the cubes it has not seen and
    /// frees the longest unused ones above [`RESIDENT_CUBES`].
    pub fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, grade: Option<&ColourGrade>) {
        let grade = grade.map(ColourGrade::sanitized).filter(|g| g.strength > 0.0);
        let Some(grade) = grade else {
            self.active = None;
            return;
        };
        self.tick += 1;
        let from = self.touch(device, queue, &grade.from);
        let to = grade.to.as_ref().map(|cube| self.touch(device, queue, cube));
        self.active = Some(Active { from, to, mix: grade.mix, strength: grade.strength });
        self.evict(from, to);
    }

    /// The id of `cube`, uploaded (once) and marked as used now.
    fn touch(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, cube: &Arc<ColourCube>) -> u64 {
        let tick = self.tick;
        self.cubes
            .entry(cube.id())
            .or_insert_with(|| Resident { view: upload(device, queue, cube), last_used: tick })
            .last_used = tick;
        cube.id()
    }

    fn evict(&mut self, keep_a: u64, keep_b: Option<u64>) {
        while self.cubes.len() > RESIDENT_CUBES {
            let oldest = self
                .cubes
                .iter()
                .filter(|(id, _)| **id != keep_a && Some(**id) != keep_b)
                .min_by_key(|(_, resident)| resident.last_used)
                .map(|(id, _)| *id);
            let Some(id) = oldest else { return };
            self.cubes.remove(&id);
        }
    }

    /// Record the grade of `io.input` into `io.output`. With no grade set it records nothing, so a caller must keep an
    /// inactive pass out of its chain.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        io: &ColourCubeIo<'_>,
    ) {
        let Some(active) = &self.active else { return };
        let Some(first) = self.cubes.get(&active.from) else { return };
        let second = active.to.and_then(|id| self.cubes.get(&id)).unwrap_or(first);
        let block = Block { a: [active.strength, active.mix, f32::from(u8::from(active.to.is_some())), 0.0] };
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&block));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour cube"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(io.input) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&first.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&second.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: self.params.as_entire_binding() },
            ],
        });
        let pipeline = self.pipelines.entry(io.output_format).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("colour cube"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: io.output_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: self.cache.as_ref(),
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("colour cube"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: io.output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The cube as an RGBA8 3D texture: red along x, green along y, blue along z.
fn upload(device: &wgpu::Device, queue: &wgpu::Queue, cube: &ColourCube) -> wgpu::TextureView {
    let n = cube.size();
    let size = wgpu::Extent3d { width: n, height: n, depth_or_array_layers: n };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour cube"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &cube.rgba(),
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(n * 4), rows_per_image: Some(n) },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D3),
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "colour_cube_tests.rs"]
mod tests;
