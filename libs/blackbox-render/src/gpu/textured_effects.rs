//! Textured world particles, depth-tested with reusable per-batch vertex buffers.

use std::collections::HashMap;

use super::effects::{ATTRIBUTES, Batch};
use super::resources::{DEPTH_FORMAT, Shared};
use super::slots::Slots;
use super::targets::write_mask;
use crate::{BlendMode, EffectVertex, TextureHandle, TexturedEffect};

pub(super) struct TexturedEffects {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the effects are drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 2]>,
    format: wgpu::TextureFormat,
    batches: Vec<(Batch, TextureHandle, BlendMode)>,
    count: usize,
}

impl TexturedEffects {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, shared: &Shared) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("textured world effects"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/effects.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("textured world effects"),
            bind_group_layouts: &[Some(&shared.globals_layout), Some(&shared.texture_layout)],
            immediate_size: 0,
        });
        let mut effects = Self { shader, layout, pipelines: HashMap::new(), format, batches: Vec::new(), count: 0 };
        effects.use_format(device, format);
        effects
    }

    /// Draw into `format` from now on, building its pipelines the first time.
    pub(super) fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        if !self.pipelines.contains_key(&format) {
            let pipelines = self.build(device, format);
            self.pipelines.insert(format, pipelines);
        }
    }

    fn build(&self, device: &wgpu::Device, format: wgpu::TextureFormat) -> [wgpu::RenderPipeline; 2] {
        std::array::from_fn(|i| {
            let blend = match i {
                0 => wgpu::BlendState::ALPHA_BLENDING,
                _ => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                },
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("textured world effects"),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<EffectVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some(["fs_textured_alpha", "fs_textured"][i]),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: write_mask(format),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layers: &[TexturedEffect]) {
        self.count = layers.len();
        for (i, layer) in layers.iter().enumerate() {
            if i == self.batches.len() {
                self.batches.push((Batch::new(device), layer.texture, layer.blend));
            }
            let (batch, texture, blend) = &mut self.batches[i];
            *texture = layer.texture;
            *blend = layer.blend;
            batch.upload(device, queue, &layer.vertices);
        }
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, textures: &Slots<wgpu::BindGroup>) {
        let Some(pipelines) = self.pipelines.get(&self.format) else { return };
        for (batch, texture, blend) in self.batches.iter().take(self.count) {
            if batch.count == 0 {
                continue;
            }
            let Some(texture) = textures.get(texture.0) else { continue };
            pass.set_pipeline(&pipelines[usize::from(*blend == BlendMode::Additive)]);
            pass.set_bind_group(1, texture, &[]);
            pass.set_vertex_buffer(0, batch.buffer.slice(..));
            pass.draw(0..batch.count, 0..1);
        }
    }
}
