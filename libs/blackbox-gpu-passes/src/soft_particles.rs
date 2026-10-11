//! Depth-aware procedural particles. Algorithm spec: docs/specs/high-quality-smoke.md.
//!
//! The pass samples the completed opaque scene depth (reverse-Z) to fade a particle where it meets
//! geometry, so the depth image must not be attached to the pass that draws the particles.

use std::collections::HashMap;

use blackbox_gfx::{DEFAULT_SOFT_DISTANCE, EffectVertex};

use crate::world::{EFFECT_VERTEX_ATTRIBUTES, WorldBindings, write_mask};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    inverse_view_proj: [[f32; 4]; 4],
    fade: [f32; 4],
}

/// The soft-particle pipeline and its depth binding. Call [`Self::prepare`] with the scene depth, then
/// [`Self::bind`] inside a render pass that targets the colour image, set the particle vertex buffer
/// (`EffectVertex` layout) and draw.
pub struct SoftParticles {
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the particles are drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    parameters: wgpu::Buffer,
    binding: Option<(wgpu::TextureView, wgpu::BindGroup)>,
}

impl SoftParticles {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, bindings: &WorldBindings) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("soft particle depth"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
        let parameters = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("soft particle parameters"),
            size: std::mem::size_of::<Parameters>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("soft particles"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("procedural soft particles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/soft_particles.wgsl").into()),
        });
        let mut soft =
            Self { shader, pipeline_layout, pipelines: HashMap::new(), format, layout, parameters, binding: None };
        soft.use_format(device, format);
        soft
    }

    /// Draw into `format` from now on, building its pipeline the first time.
    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        if self.pipelines.contains_key(&format) {
            return;
        }
        let entry_point = if crate::world::is_srgb(format) { "fs_main_srgb" } else { "fs_main" };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("soft particles"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<EffectVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &EFFECT_VERTEX_ATTRIBUTES,
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: write_mask(format),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.insert(format, pipeline);
    }

    /// Upload the fade distance and the inverse view-projection, and bind `depth` (the scene's reverse-Z
    /// depth view, sampleable). `distance` is in world units; a non-finite value falls back to the default.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        depth: &wgpu::TextureView,
        inverse: glam::Mat4,
        distance: f32,
    ) {
        let distance = if distance.is_finite() { distance.max(0.001) } else { DEFAULT_SOFT_DISTANCE };
        let parameters = Parameters { inverse_view_proj: inverse.to_cols_array_2d(), fade: [distance, 0.0, 0.0, 0.0] };
        queue.write_buffer(&self.parameters, 0, bytemuck::bytes_of(&parameters));
        if self.binding.as_ref().is_some_and(|(view, _)| view == depth) {
            return;
        }
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("soft particle depth"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(depth) },
                wgpu::BindGroupEntry { binding: 1, resource: self.parameters.as_entire_binding() },
            ],
        });
        self.binding = Some((depth.clone(), binding));
    }

    /// Set the pipeline and bind groups 0 (`globals`, the group of [`WorldBindings`]) and 1 (the depth).
    /// Panics if [`Self::prepare`] has not run.
    pub fn bind(&self, pass: &mut wgpu::RenderPass<'_>, globals: &wgpu::BindGroup) {
        if let Some(pipeline) = self.pipelines.get(&self.format) {
            pass.set_pipeline(pipeline);
        }
        pass.set_bind_group(0, globals, &[]);
        pass.set_bind_group(1, &self.binding.as_ref().expect("prepared soft particle depth").1, &[]);
    }
}
