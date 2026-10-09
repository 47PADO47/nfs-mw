//! The FSR 1 passes (AMD FidelityFX Super Resolution 1): EASU upscales an image to the output size and
//! RCAS sharpens it. Both write the output size, so they belong last in a chain, after tone mapping and
//! anti-aliasing and right before the final copy.
//!
//! The input is expected to be anti-aliased and display-referred (gamma-like, what the final copy writes
//! to the surface); the shader clamps it to 0..1. The shader is in `shaders/fsr1.wgsl`.
//!
//! From any render loop: build [`fsr1_passes`] once, then per frame call [`Fsr1Pass::encode`] on EASU
//! (render-size image in, output-size image out) and, when the sharpening pass exists, on RCAS (that
//! output in, a second output-size image out).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// The name of the upscaling pass (for logs and pass lists).
pub const FSR1_EASU: &str = "fsr1-easu";
/// The name of the sharpening pass.
pub const FSR1_RCAS: &str = "fsr1-rcas";

/// The RCAS lobe scale (`exp2(-stops)`), shared between the caller and the sharpening pass so a
/// sharpness change needs no new pipeline.
#[derive(Clone)]
pub struct RcasScale(Arc<AtomicU32>);

impl RcasScale {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU32::new(1.0f32.to_bits())))
    }

    /// Set the RCAS attenuation in stops (0 is the sharpest).
    pub fn set_stops(&self, stops: f32) {
        self.0.store((-stops).exp2().to_bits(), Ordering::Relaxed);
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

impl Default for RcasScale {
    fn default() -> Self {
        Self::new()
    }
}

/// Which half of FSR 1 a [`Fsr1Pass`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fsr1Stage {
    Easu,
    Rcas,
}

impl Fsr1Stage {
    /// [`FSR1_EASU`] or [`FSR1_RCAS`].
    pub fn name(self) -> &'static str {
        match self {
            Self::Easu => FSR1_EASU,
            Self::Rcas => FSR1_RCAS,
        }
    }

    fn entry(self) -> &'static str {
        match self {
            Self::Easu => "fs_easu",
            Self::Rcas => "fs_rcas",
        }
    }
}

/// The `Params` uniform of `fsr1.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    out_size: [f32; 2],
    rcas: f32,
    pad: f32,
}

/// One FSR 1 stage with its pipeline cache and parameter buffer.
pub struct Fsr1Pass {
    stage: Fsr1Stage,
    shader: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    params: wgpu::Buffer,
    rcas: RcasScale,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}

/// The upscale pass, then the sharpening pass when `sharpen` is set.
pub fn fsr1_passes(device: &wgpu::Device, rcas: &RcasScale, sharpen: bool) -> Vec<Fsr1Pass> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fsr1"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fsr1.wgsl").into()),
    });
    let stages: &[Fsr1Stage] = if sharpen { &[Fsr1Stage::Easu, Fsr1Stage::Rcas] } else { &[Fsr1Stage::Easu] };
    stages.iter().map(|&stage| Fsr1Pass::new(device, &shader, stage, rcas)).collect()
}

impl Fsr1Pass {
    fn new(device: &wgpu::Device, shader: &wgpu::ShaderModule, stage: Fsr1Stage, rcas: &RcasScale) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(stage.name()),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(stage.name()),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(stage.name()),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            stage,
            shader: shader.clone(),
            layout,
            pipeline_layout,
            params,
            rcas: rcas.clone(),
            pipelines: HashMap::new(),
        }
    }

    fn pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        self.pipelines.entry(format).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.stage.name()),
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
                    entry_point: Some(self.stage.entry()),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }
}

/// The images one FSR 1 stage reads and writes.
pub struct Fsr1Io<'a> {
    /// The image to read, any size; the stage reads it with exact texel loads.
    pub input: &'a wgpu::TextureView,
    /// The image to write, `output_size` pixels, a render attachment of `output_format`.
    pub output: &'a wgpu::TextureView,
    pub output_format: wgpu::TextureFormat,
    pub output_size: (u32, u32),
}

impl Fsr1Pass {
    pub fn stage(&self) -> Fsr1Stage {
        self.stage
    }

    /// [`FSR1_EASU`] or [`FSR1_RCAS`].
    pub fn name(&self) -> &'static str {
        self.stage.name()
    }

    /// Record the stage: one render pass over `io.output` that clears it and draws the filtered input.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        io: &Fsr1Io<'_>,
    ) {
        let params =
            Params { out_size: [io.output_size.0 as f32, io.output_size.1 as f32], rcas: self.rcas.get(), pad: 0.0 };
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.stage.name()),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(io.input) },
                wgpu::BindGroupEntry { binding: 1, resource: self.params.as_entire_binding() },
            ],
        });
        let name = self.stage.name();
        let pipeline = self.pipeline(device, io.output_format);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(name),
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
